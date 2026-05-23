#[derive(Debug, Clone)]
pub struct ExecPolicyDecision {
    pub allow: bool,
    pub requires_approval: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ExecPolicyContext<'a> {
    pub command: &'a str,
    pub cwd: &'a str,
    pub sandbox_mode: Option<&'a str>,
}

pub struct ExecPolicyEngine {}

impl ExecPolicyEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn check(&self, ctx: ExecPolicyContext) -> ExecPolicyDecision {
        if ctx.command.starts_with("rm -rf") || ctx.command.starts_with("del /f /s /q") {
            return ExecPolicyDecision {
                allow: false,
                requires_approval: false,
                reason: "Dangerous delete command blocked".to_string(),
            };
        }

        ExecPolicyDecision {
            allow: true,
            requires_approval: false,
            reason: String::new(),
        }
    }
}

impl Default for ExecPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
