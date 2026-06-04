use floral_notepaper_lib::services::{addon::OperationService, notes::AppError};
use serde_json::{json, Value};
use std::io::{self, Read};

pub fn run(service: Result<OperationService, AppError>, operation: &str) {
    let result = service.and_then(|service| service.execute(operation, read_input()?));
    match result {
        Ok(data) => println!("{}", json!({ "ok": true, "data": data })),
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "error": {
                        "code": error.code,
                        "message": error.message,
                        "details": error.details,
                    }
                })
            );
            std::process::exit(1);
        }
    }
}

fn read_input() -> Result<Value, AppError> {
    let mut raw = String::new();
    io::stdin().read_to_string(&mut raw)?;
    if raw.trim().is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        serde_json::from_str(&raw).map_err(Into::into)
    }
}
