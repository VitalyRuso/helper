use shared_contracts::AssistantTestRun;

pub fn passed(run: &AssistantTestRun) -> bool {
    run.passed
}
