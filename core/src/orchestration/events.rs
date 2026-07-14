//! UiEvent + audit payloads for orchestration lifecycle.

use serde_json::{Value, json};

use crate::orchestration::recipe::RecipeStep;
use crate::orchestration::run::{OrchestrationRunDto, OrchestrationRunMeta};

pub fn orchestration_started(run: &OrchestrationRunMeta) -> Value {
    json!({
        "type": "orchestration_started",
        "run_id": run.id,
        "recipe_id": run.recipe_id,
        "source_conversation_id": run.source_conversation_id,
    })
}

pub fn orchestration_step_started(run: &OrchestrationRunMeta, step: &RecipeStep) -> Value {
    json!({
        "type": "orchestration_step_started",
        "run_id": run.id,
        "step_index": run.current_step,
        "step_type": step_type_name(step),
    })
}

pub fn orchestration_forked(
    run: &OrchestrationRunMeta,
    conversation_id: &str,
    harness_id: &str,
    model_id: &str,
) -> Value {
    json!({
        "type": "orchestration_forked",
        "run_id": run.id,
        "conversation_id": conversation_id,
        "harness_id": harness_id,
        "model_id": model_id,
    })
}

pub fn orchestration_branch_completed(
    run: &OrchestrationRunMeta,
    conversation_id: &str,
    reason: &str,
) -> Value {
    json!({
        "type": "orchestration_branch_completed",
        "run_id": run.id,
        "conversation_id": conversation_id,
        "reason": reason,
    })
}

pub fn orchestration_finished(dto: &OrchestrationRunDto) -> Value {
    json!({
        "type": "orchestration_finished",
        "run": dto,
    })
}

fn step_type_name(step: &RecipeStep) -> &'static str {
    match step {
        RecipeStep::ForkRun { .. } => "fork_run",
        RecipeStep::Parallel { .. } => "parallel",
        RecipeStep::Loop { .. } => "loop",
    }
}
