use super::errors;
use crate::services::notes::AppError;
use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "floral-notepaper";
const MIRROR_CDK_ACCOUNT: &str = "mirrorchyan-cdk";

#[derive(Debug, Clone)]
pub struct CdkStore {
    service: &'static str,
    account: &'static str,
}

impl Default for CdkStore {
    fn default() -> Self {
        Self {
            service: SERVICE_NAME,
            account: MIRROR_CDK_ACCOUNT,
        }
    }
}

impl CdkStore {
    pub fn has_cdk(&self) -> Result<bool, AppError> {
        match self.entry()?.get_password() {
            Ok(cdk) => Ok(!cdk.trim().is_empty()),
            Err(KeyringError::NoEntry) => Ok(false),
            Err(error) => Err(errors::secure_store_unavailable(error)),
        }
    }

    pub fn set_cdk(&self, cdk: &str) -> Result<(), AppError> {
        let cdk = cdk.trim();
        if cdk.is_empty() {
            return Err(errors::app_error(
                "mirrorCdkEmpty",
                "Mirror 酱 CDK 不能为空",
            ));
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
        Entry::new(self.service, self.account).map_err(errors::secure_store_unavailable)
    }
}
