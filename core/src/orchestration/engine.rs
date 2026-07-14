//! Prompt-free recipe execution: fork, send user-authored messages, wait for turns.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::oneshot;

use crate::app::TamtriCore;
use crate::conversation::Id;
use crate::harness::TurnEndReason;
use crate::orchestration::events::{
    orchestration_branch_completed, orchestration_forked, orchestration_step_started,
};
use crate::orchestration::recipe::{ParallelBranch, Recipe, RecipeStep, apply_template};
use crate::orchestration::run::OrchestrationRunMeta;
use crate::orchestration::store;
use crate::{CoreError, Result};

pub fn execute(
    core: &TamtriCore,
    recipe: &Recipe,
    run: &mut OrchestrationRunMeta,
    inputs: &HashMap<String, String>,
) -> Result<()> {
    for (index, step) in recipe.steps.iter().enumerate() {
        run.current_step = index as u32;
        run.touch();
        store::save_run(core.vault_root(), run)?;
        execute_step_sync(core, step, run, inputs)?;
    }
    Ok(())
}

pub async fn execute_async(
    core: &TamtriCore,
    recipe: &Recipe,
    run: &mut OrchestrationRunMeta,
    inputs: &HashMap<String, String>,
    cancel: &Arc<AtomicBool>,
) -> Result<()> {
    for (index, step) in recipe.steps.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err(CoreError::Protocol("orchestration cancelled".to_string()));
        }
        run.current_step = index as u32;
        run.touch();
        store::save_run(core.vault_root(), run)?;
        core.emit_orchestration_ui(
            run,
            "orchestration_step_started",
            orchestration_step_started(run, step),
        );
        execute_step_async(core, step, run, inputs, cancel).await?;
    }
    Ok(())
}

fn execute_step_sync(
    core: &TamtriCore,
    step: &RecipeStep,
    run: &mut OrchestrationRunMeta,
    inputs: &HashMap<String, String>,
) -> Result<()> {
    match step {
        RecipeStep::ForkRun {
            harness_id,
            model_id,
            message,
        } => {
            let text = apply_template(message, inputs);
            let fork =
                core.fork_conversation_inner(&run.latest_conversation_id, harness_id, model_id)?;
            let reason = core.send_message_and_wait_inner(&fork.id, &text)?;
            if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
                return Err(CoreError::Protocol(format!(
                    "fork_run ended with {reason:?}"
                )));
            }
            run.latest_conversation_id = fork.id;
            run.branch_conversation_ids.clear();
            run.touch();
            store::save_run(core.vault_root(), run)?;
            Ok(())
        }
        RecipeStep::Parallel { branches } => run_parallel_sync(core, run, branches, inputs),
        RecipeStep::Loop {
            max_iterations,
            harness_id,
            model_id,
            message,
        } => {
            let text = apply_template(message, inputs);
            for _ in 0..*max_iterations {
                let fork = core.fork_conversation_inner(
                    &run.latest_conversation_id,
                    harness_id,
                    model_id,
                )?;
                let reason = core.send_message_and_wait_inner(&fork.id, &text)?;
                run.latest_conversation_id = fork.id.clone();
                run.touch();
                store::save_run(core.vault_root(), run)?;
                if matches!(reason, TurnEndReason::EndTurn) {
                    break;
                }
                if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
                    return Err(CoreError::Protocol(format!(
                        "loop iteration ended with {reason:?}"
                    )));
                }
            }
            Ok(())
        }
    }
}

async fn execute_step_async(
    core: &TamtriCore,
    step: &RecipeStep,
    run: &mut OrchestrationRunMeta,
    inputs: &HashMap<String, String>,
    cancel: &Arc<AtomicBool>,
) -> Result<()> {
    match step {
        RecipeStep::ForkRun {
            harness_id,
            model_id,
            message,
        } => {
            let text = apply_template(message, inputs);
            let fork =
                core.fork_conversation_inner(&run.latest_conversation_id, harness_id, model_id)?;
            core.emit_orchestration_ui(
                run,
                "orchestration_forked",
                orchestration_forked(run, &fork.id, harness_id, model_id),
            );
            let reason = core
                .send_message_and_wait_async(&fork.id, &text, cancel)
                .await?;
            core.emit_orchestration_ui(
                run,
                "orchestration_branch_completed",
                orchestration_branch_completed(run, &fork.id, &format!("{reason:?}")),
            );
            if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
                return Err(CoreError::Protocol(format!(
                    "fork_run ended with {reason:?}"
                )));
            }
            run.latest_conversation_id = fork.id;
            run.branch_conversation_ids.clear();
            run.touch();
            store::save_run(core.vault_root(), run)?;
            Ok(())
        }
        RecipeStep::Parallel { branches } => {
            run_parallel_async(core, run, branches, inputs, cancel).await
        }
        RecipeStep::Loop {
            max_iterations,
            harness_id,
            model_id,
            message,
        } => {
            let text = apply_template(message, inputs);
            for _ in 0..*max_iterations {
                if cancel.load(Ordering::SeqCst) {
                    return Err(CoreError::Protocol("orchestration cancelled".to_string()));
                }
                let fork = core.fork_conversation_inner(
                    &run.latest_conversation_id,
                    harness_id,
                    model_id,
                )?;
                core.emit_orchestration_ui(
                    run,
                    "orchestration_forked",
                    orchestration_forked(run, &fork.id, harness_id, model_id),
                );
                let reason = core
                    .send_message_and_wait_async(&fork.id, &text, cancel)
                    .await?;
                core.emit_orchestration_ui(
                    run,
                    "orchestration_branch_completed",
                    orchestration_branch_completed(run, &fork.id, &format!("{reason:?}")),
                );
                run.latest_conversation_id = fork.id.clone();
                run.touch();
                store::save_run(core.vault_root(), run)?;
                if matches!(reason, TurnEndReason::EndTurn) {
                    break;
                }
                if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
                    return Err(CoreError::Protocol(format!(
                        "loop iteration ended with {reason:?}"
                    )));
                }
            }
            Ok(())
        }
    }
}

fn run_parallel_sync(
    core: &TamtriCore,
    run: &mut OrchestrationRunMeta,
    branches: &[ParallelBranch],
    inputs: &HashMap<String, String>,
) -> Result<()> {
    let source = run.latest_conversation_id.clone();
    let mut forks = Vec::new();
    let mut waiters = Vec::new();

    for branch in branches {
        let fork = core.fork_conversation_inner(&source, &branch.harness_id, &branch.model_id)?;
        let id = parse_conversation_id(&fork.id)?;
        let (tx, rx) = oneshot::channel();
        core.register_turn_waiter(id, tx);
        waiters.push((fork.id.clone(), rx));
        forks.push((fork.id, branch));
    }

    for (conv_id, branch) in &forks {
        let text = apply_template(&branch.message, inputs);
        core.send_message_inner(conv_id, &text)?;
    }

    let mut branch_ids = Vec::new();
    for (conv_id, rx) in waiters {
        let reason = core.wait_turn_receiver(&conv_id, rx)?;
        if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
            return Err(CoreError::Protocol(format!(
                "parallel branch {conv_id} ended with {reason:?}"
            )));
        }
        branch_ids.push(conv_id);
    }

    run.branch_conversation_ids = branch_ids;
    run.touch();
    store::save_run(core.vault_root(), run)?;
    Ok(())
}

async fn run_parallel_async(
    core: &TamtriCore,
    run: &mut OrchestrationRunMeta,
    branches: &[ParallelBranch],
    inputs: &HashMap<String, String>,
    cancel: &Arc<AtomicBool>,
) -> Result<()> {
    let source = run.latest_conversation_id.clone();
    let mut forks = Vec::new();
    let mut waiters = Vec::new();

    for branch in branches {
        let fork = core.fork_conversation_inner(&source, &branch.harness_id, &branch.model_id)?;
        core.emit_orchestration_ui(
            run,
            "orchestration_forked",
            orchestration_forked(run, &fork.id, &branch.harness_id, &branch.model_id),
        );
        let id = parse_conversation_id(&fork.id)?;
        let (tx, rx) = oneshot::channel();
        core.register_turn_waiter(id, tx);
        waiters.push((fork.id.clone(), rx));
        forks.push((fork.id, branch));
    }

    for (conv_id, branch) in &forks {
        let text = apply_template(&branch.message, inputs);
        core.send_message_inner(conv_id, &text)?;
    }

    let mut branch_ids = Vec::new();
    for (conv_id, rx) in waiters {
        let reason = core.wait_turn_receiver_async(&conv_id, rx, cancel).await?;
        core.emit_orchestration_ui(
            run,
            "orchestration_branch_completed",
            orchestration_branch_completed(run, &conv_id, &format!("{reason:?}")),
        );
        if matches!(reason, TurnEndReason::Failed | TurnEndReason::Cancelled) {
            return Err(CoreError::Protocol(format!(
                "parallel branch {conv_id} ended with {reason:?}"
            )));
        }
        branch_ids.push(conv_id);
    }

    run.branch_conversation_ids = branch_ids;
    run.touch();
    store::save_run(core.vault_root(), run)?;
    Ok(())
}

fn parse_conversation_id(raw: &str) -> Result<Id> {
    raw.parse()
        .map_err(|err| CoreError::MalformedVault(format!("invalid conversation id: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::recipe::RECIPE_SCHEMA_VERSION;

    #[test]
    fn apply_template_in_engine_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), "hello".to_string());
        assert_eq!(apply_template("{{message}}", &inputs), "hello");
    }

    #[test]
    fn recipe_step_count_matches_run_progress() {
        let recipe = Recipe {
            schema_version: RECIPE_SCHEMA_VERSION,
            id: "test".to_string(),
            title: "Test".to_string(),
            description: None,
            steps: vec![RecipeStep::ForkRun {
                harness_id: "claude-native".to_string(),
                model_id: "default".to_string(),
                message: "hi".to_string(),
            }],
        };
        assert_eq!(recipe.steps.len(), 1);
    }
}
