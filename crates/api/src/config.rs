use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentEnvironment {
    Local,
    Staging,
    Production,
}

impl DeploymentEnvironment {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => Err("API_ENV must be exactly local, staging, or production".into()),
        }
    }

    fn is_local(self) -> bool {
        matches!(self, Self::Local)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentConfig {
    pub environment: DeploymentEnvironment,
    pub simulation_mode: bool,
    allow_insecure_local_rpc: bool,
}

impl DeploymentConfig {
    pub fn from_env() -> Result<Self, String> {
        let environment = env::var("API_ENV")
            .map_err(|_| "API_ENV is required and must identify the deployment environment")?;
        let simulation_mode = parse_bool_env("API_SIMULATION_MODE", false)?;
        let allow_insecure_local_rpc = parse_bool_env("API_ALLOW_INSECURE_LOCAL_RPC", false)?;
        Self::from_values(&environment, simulation_mode, allow_insecure_local_rpc)
    }

    pub fn from_values(
        environment: &str,
        simulation_mode: bool,
        allow_insecure_local_rpc: bool,
    ) -> Result<Self, String> {
        let environment = DeploymentEnvironment::parse(environment)?;
        if simulation_mode && !environment.is_local() {
            return Err("API_SIMULATION_MODE=true is permitted only in API_ENV=local".into());
        }
        if allow_insecure_local_rpc && !environment.is_local() {
            return Err(
                "API_ALLOW_INSECURE_LOCAL_RPC=true is permitted only in API_ENV=local".into(),
            );
        }
        Ok(Self {
            environment,
            simulation_mode,
            allow_insecure_local_rpc,
        })
    }

    pub fn allows_insecure_local_rpc(self) -> bool {
        self.environment.is_local() && self.allow_insecure_local_rpc
    }
}

fn parse_bool_env(name: &str, default: bool) -> Result<bool, String> {
    match env::var(name) {
        Ok(value) => parse_bool_value(name, &value),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(format!("{name} must be valid UTF-8")),
    }
}

fn parse_bool_value(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{name} must be exactly true or false")),
    }
}

#[cfg(test)]
mod tests {
    use super::{DeploymentConfig, DeploymentEnvironment, parse_bool_value};

    #[test]
    fn local_development_can_explicitly_enable_simulation_and_insecure_rpc() {
        let config = DeploymentConfig::from_values("local", true, true).unwrap();
        assert_eq!(config.environment, DeploymentEnvironment::Local);
        assert!(config.simulation_mode);
        assert!(config.allows_insecure_local_rpc());
    }

    #[test]
    fn staging_and_production_reject_simulation_and_insecure_rpc() {
        for environment in ["staging", "production"] {
            assert!(DeploymentConfig::from_values(environment, true, false).is_err());
            assert!(DeploymentConfig::from_values(environment, false, true).is_err());
            let config = DeploymentConfig::from_values(environment, false, false).unwrap();
            assert!(!config.simulation_mode);
            assert!(!config.allows_insecure_local_rpc());
        }
    }

    #[test]
    fn unknown_environment_and_invalid_booleans_fail_closed() {
        assert!(DeploymentConfig::from_values("", false, false).is_err());
        assert!(DeploymentConfig::from_values("mainnet", false, false).is_err());
        assert!(parse_bool_value("API_TEST_INVALID_BOOLEAN", "yes").is_err());
    }
}
