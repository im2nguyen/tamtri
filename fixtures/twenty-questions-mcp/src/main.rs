//! Stdio MCP server that plays 20 Questions via form-mode elicitation.
//! Used to exercise tamtri gateway elicitation routing (Milestone 6).

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde_json::{Value, json};

const WORDS: &[&str] = &["apple", "elephant", "bicycle", "guitar", "umbrella"];
const MAX_TURNS: u8 = 20;

static GAMES: OnceLock<Mutex<HashMap<u64, GameState>>> = OnceLock::new();
static NEXT_GAME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct WordProfile {
    is_animal: bool,
    is_fruit: bool,
    is_vehicle: bool,
    is_instrument: bool,
    is_large: bool,
    has_wheels: bool,
    is_musical: bool,
    used_outdoors: bool,
}

impl WordProfile {
    fn for_word(word: &str) -> Self {
        match word {
            "apple" => Self {
                is_animal: false,
                is_fruit: true,
                is_vehicle: false,
                is_instrument: false,
                is_large: false,
                has_wheels: false,
                is_musical: false,
                used_outdoors: false,
            },
            "elephant" => Self {
                is_animal: true,
                is_fruit: false,
                is_vehicle: false,
                is_instrument: false,
                is_large: true,
                has_wheels: false,
                is_musical: false,
                used_outdoors: true,
            },
            "bicycle" => Self {
                is_animal: false,
                is_fruit: false,
                is_vehicle: true,
                is_instrument: false,
                is_large: false,
                has_wheels: true,
                is_musical: false,
                used_outdoors: true,
            },
            "guitar" => Self {
                is_animal: false,
                is_fruit: false,
                is_vehicle: false,
                is_instrument: true,
                is_large: false,
                has_wheels: false,
                is_musical: true,
                used_outdoors: false,
            },
            "umbrella" => Self {
                is_animal: false,
                is_fruit: false,
                is_vehicle: false,
                is_instrument: false,
                is_large: false,
                has_wheels: false,
                is_musical: false,
                used_outdoors: true,
            },
            _ => Self {
                is_animal: false,
                is_fruit: false,
                is_vehicle: false,
                is_instrument: false,
                is_large: false,
                has_wheels: false,
                is_musical: false,
                used_outdoors: false,
            },
        }
    }
}

#[derive(Debug, Clone)]
struct GameState {
    secret: String,
    turns_used: u8,
}

fn games() -> &'static Mutex<HashMap<u64, GameState>> {
    GAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn seeded_word_index() -> usize {
    let seed = std::env::var("TWENTY_QUESTIONS_SEED")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(7);
    word_index_for_seed(seed)
}

fn word_index_for_seed(seed: u64) -> usize {
    let mut state = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    state ^= state >> 17;
    state = state.wrapping_mul(0x85EB_CA6B);
    state ^= state >> 13;
    (state as usize) % WORDS.len()
}

fn start_game() -> Value {
    let game_id = NEXT_GAME_ID.fetch_add(1, Ordering::Relaxed);
    let secret = WORDS[seeded_word_index()].to_string();
    games()
        .lock()
        .expect("game lock")
        .insert(
            game_id,
            GameState {
                secret,
                turns_used: 0,
            },
        );
    tool_ok(
        "I'm thinking of something. Ask up to 20 yes/no questions, then make a guess.",
        json!({
            "gameId": game_id,
            "turnsRemaining": MAX_TURNS,
            "status": "in_progress"
        }),
    )
}

fn answer_question(secret: &str, question: &str) -> &'static str {
    let q = question.to_ascii_lowercase();
    let profile = WordProfile::for_word(secret);

    let rules: [(&[&str], bool); 8] = [
        (&["animal", "creature", "pet"], profile.is_animal),
        (&["fruit", "food", "edible", "eat"], profile.is_fruit),
        (&["vehicle", "transport", "ride"], profile.is_vehicle),
        (&["instrument", "music"], profile.is_instrument),
        (&["large", "big", "heavy"], profile.is_large),
        (&["wheel", "wheels"], profile.has_wheels),
        (&["musical", "play music", "make music"], profile.is_musical),
        (&["outdoor", "outside", "rain", "weather"], profile.used_outdoors),
    ];

    for (needles, value) in rules {
        if needles.iter().any(|needle| q.contains(needle)) {
            return if value { "yes" } else { "no" };
        }
    }

    if q.contains(secret) {
        return "yes";
    }

    "maybe"
}

fn game_state(game_id: u64) -> Result<GameState, String> {
    games()
        .lock()
        .expect("game lock")
        .get(&game_id)
        .cloned()
        .ok_or_else(|| format!("unknown game id: {game_id}"))
}

fn update_game(game_id: u64, update: impl FnOnce(&mut GameState)) -> Result<GameState, String> {
    let mut map = games().lock().expect("game lock");
    let state = map
        .get_mut(&game_id)
        .ok_or_else(|| format!("unknown game id: {game_id}"))?;
    update(state);
    Ok(state.clone())
}

fn submit_question(
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
    game_id: u64,
) -> Result<Value, String> {
    let mut state = game_state(game_id)?;
    if state.turns_used >= MAX_TURNS {
        return Ok(tool_error("No questions left. Make a guess instead."));
    }

    let elicitation = elicit_form(
        stdout,
        input,
        "elicit-question",
        "Ask a yes/no question about the secret thing.",
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "title": "Your question",
                    "description": "Phrase it so the answer can be yes, no, or maybe.",
                    "minLength": 3
                }
            },
            "required": ["question"]
        }),
    )?;

    let ElicitationOutcome::Accepted(content) = elicitation else {
        return Ok(tool_ok(
            format!("Question skipped ({elicitation})."),
            json!({ "gameId": game_id, "status": "waiting" }),
        ));
    };

    let question = content
        .get("question")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing question field".to_string())?;
    let answer = answer_question(&state.secret, question);
    state = update_game(game_id, |game| {
        game.turns_used += 1;
    })?;
    let turns_remaining = MAX_TURNS.saturating_sub(state.turns_used);

    Ok(tool_ok(
        format!("Q: {question}\nA: {answer}\nTurns remaining: {turns_remaining}"),
        json!({
            "gameId": game_id,
            "question": question,
            "answer": answer,
            "turnsRemaining": turns_remaining,
            "status": if turns_remaining == 0 { "must_guess" } else { "in_progress" }
        }),
    ))
}

fn make_guess(
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
    game_id: u64,
) -> Result<Value, String> {
    let state = game_state(game_id)?;

    let elicitation = elicit_form(
        stdout,
        input,
        "elicit-guess",
        "What is your final guess?",
        json!({
            "type": "object",
            "properties": {
                "guess": {
                    "type": "string",
                    "title": "Your guess",
                    "minLength": 2
                }
            },
            "required": ["guess"]
        }),
    )?;

    let ElicitationOutcome::Accepted(content) = elicitation else {
        return Ok(tool_ok(
            format!("Guess skipped ({elicitation})."),
            json!({ "gameId": game_id, "status": "waiting" }),
        ));
    };

    let guess = content
        .get("guess")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing guess field".to_string())?;
    let won = guess.eq_ignore_ascii_case(&state.secret);
    games().lock().expect("game lock").remove(&game_id);

    if won {
        Ok(tool_ok(
            format!("Correct! The word was \"{}\".", state.secret),
            json!({ "gameId": game_id, "guess": guess, "status": "won", "secret": state.secret }),
        ))
    } else {
        Ok(tool_ok(
            format!("Not quite. The word was \"{}\".", state.secret),
            json!({ "gameId": game_id, "guess": guess, "status": "lost", "secret": state.secret }),
        ))
    }
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut stdout = io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        if matches!(input.read_line(&mut line), Ok(0) | Err(_)) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(line.trim_end()) else {
            continue;
        };
        if message.get("id").is_none() {
            continue;
        }

        let response = match message.get("method").and_then(Value::as_str) {
            Some("initialize") => response(
                &message,
                json!({
                    "protocolVersion": "2025-11-25",
                    "capabilities": {
                        "tools": {"listChanged": false}
                    },
                    "serverInfo": {"name": "twenty-questions-mcp", "version": "0.1.0"}
                }),
            ),
            Some("tools/list") => response(
                &message,
                json!({
                    "tools": [
                        {
                            "name": "start_game",
                            "description": "Start a new 20 Questions round.",
                            "inputSchema": {"type": "object", "properties": {}}
                        },
                        {
                            "name": "submit_question",
                            "description": "Ask one yes/no question via elicitation.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "gameId": {"type": "integer", "description": "Game id from start_game"}
                                },
                                "required": ["gameId"]
                            }
                        },
                        {
                            "name": "make_guess",
                            "description": "Submit your final guess via elicitation.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "gameId": {"type": "integer", "description": "Game id from start_game"}
                                },
                                "required": ["gameId"]
                            }
                        }
                    ]
                }),
            ),
            Some("tools/call") => {
                let tool_name = message
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let arguments = message
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let result = match tool_name {
                    "start_game" => Ok(start_game()),
                    "submit_question" => match arguments.get("gameId").and_then(Value::as_u64) {
                        Some(game_id) => submit_question(&mut stdout, &mut input, game_id),
                        None => Err("gameId is required".to_string()),
                    },
                    "make_guess" => match arguments.get("gameId").and_then(Value::as_u64) {
                        Some(game_id) => make_guess(&mut stdout, &mut input, game_id),
                        None => Err("gameId is required".to_string()),
                    },
                    _ => Err(format!("unknown tool: {tool_name}")),
                };
                match result {
                    Ok(value) => response(&message, value),
                    Err(err) => error_response(&message, -32000, err),
                }
            }
            Some(method) => error_response(&message, -32601, format!("unknown method: {method}")),
            None => error_response(&message, -32600, "missing method"),
        };

        if serde_json::to_writer(&mut stdout, &response).is_err() {
            break;
        }
        if writeln!(stdout).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ElicitationOutcome {
    Accepted(Value),
    Declined,
    Cancelled,
}

impl std::fmt::Display for ElicitationOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accepted(_) => write!(f, "accepted"),
            Self::Declined => write!(f, "declined"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

fn elicit_form(
    stdout: &mut io::Stdout,
    input: &mut impl BufRead,
    request_id: &str,
    message: &str,
    requested_schema: Value,
) -> Result<ElicitationOutcome, String> {
    let request = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "elicitation/create",
        "params": {
            "mode": "form",
            "message": message,
            "requestedSchema": requested_schema
        }
    });
    serde_json::to_writer(&mut *stdout, &request).map_err(|err| err.to_string())?;
    writeln!(stdout).map_err(|err| err.to_string())?;
    stdout.flush().map_err(|err| err.to_string())?;

    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|err| err.to_string())?;
    if line.trim().is_empty() {
        return Err("missing elicitation response".to_string());
    }
    let response: Value = serde_json::from_str(line.trim_end()).map_err(|err| err.to_string())?;
    if response.get("error").is_some() {
        return Err(format!("elicitation failed: {response}"));
    }
    let action = response
        .pointer("/result/action")
        .and_then(Value::as_str)
        .unwrap_or("cancel");
    match action {
        "accept" => Ok(ElicitationOutcome::Accepted(
            response
                .pointer("/result/content")
                .cloned()
                .unwrap_or_else(|| json!({})),
        )),
        "decline" => Ok(ElicitationOutcome::Declined),
        _ => Ok(ElicitationOutcome::Cancelled),
    }
}

fn tool_ok(text: impl Into<String>, structured: Value) -> Value {
    json!({
        "content": [{"type": "text", "text": text.into()}],
        "isError": false,
        "structuredContent": structured
    })
}

fn tool_error(text: impl Into<String>) -> Value {
    json!({
        "content": [{"type": "text", "text": text.into()}],
        "isError": true
    })
}

fn response(request: &Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request.get("id").cloned().unwrap_or(Value::Null),
        "result": result
    })
}

fn error_response(request: &Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": request.get("id").cloned().unwrap_or(Value::Null),
        "error": {"code": code, "message": message.into()}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_word_is_deterministic() {
        assert_eq!(word_index_for_seed(42), word_index_for_seed(42));
    }

    #[test]
    fn answer_question_handles_animals() {
        assert_eq!(answer_question("elephant", "Is it an animal?"), "yes");
        assert_eq!(answer_question("apple", "Is it an animal?"), "no");
    }

    #[test]
    fn answer_question_handles_fruit() {
        assert_eq!(answer_question("apple", "Is it a fruit?"), "yes");
        assert_eq!(answer_question("guitar", "Is it a fruit?"), "no");
    }
}
