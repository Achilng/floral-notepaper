use super::errors;
use crate::services::notes::AppError;
use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "floral-notepaper";
const MIRROR_CDK_ACCOUNT: &str = "mirrorchyan-cdk";

#[derive(Debug, Clone)]
pub struct CdkStore {
    service: &'static str,
    account: &'static str,
    #[cfg(test)]
    unavailable: bool,
}

impl Default for CdkStore {
    fn default() -> Self {
        Self {
            service: SERVICE_NAME,
            account: MIRROR_CDK_ACCOUNT,
            #[cfg(test)]
            unavailable: false,
        }
    }
}

impl CdkStore {
    #[cfg(test)]
    pub(crate) fn invalid_for_tests() -> Self {
        Self {
            service: "",
            account: "",
            unavailable: true,
        }
    }

    pub fn has_cdk(&self) -> Result<bool, AppError> {
        match self.entry()?.get_password() {
            Ok(cdk) => Ok(!cdk.trim().is_empty()),
            Err(KeyringError::NoEntry) => Ok(false),
            Err(error) => Err(errors::secure_store_unavailable(error)),
        }
    }

    pub fn get_cdk(&self) -> Option<String> {
        self.entry()
            .ok()
            .and_then(|entry| entry.get_password().ok())
            .map(|cdk| cdk.trim().to_string())
            .filter(|cdk| !cdk.is_empty())
    }

    pub fn set_cdk(&self, cdk: &str) -> Result<(), AppError> {
        let cdk = cdk.trim();
        if cdk.is_empty() {
            return Err(errors::app_error("mirrorCdkEmpty", "Mirror酱 CDK 不能为空"));
        }

        self.entry()?
            .set_password(cdk)
            .map_err(errors::secure_store_unavailable)
    }

    pub fn clear_cdk(&self) -> Result<(), AppError> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(errors::secure_store_unavailable(error)),
        }
    }

    fn entry(&self) -> Result<Entry, AppError> {
        #[cfg(test)]
        if self.unavailable {
            return Err(errors::secure_store_unavailable(
                "test secure store unavailable",
            ));
        }

        Entry::new(self.service, self.account).map_err(errors::secure_store_unavailable)
    }
}
