// Health check models

#[derive(Default, serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum StatusEnum {
    Up,

    #[default]
    Down,
}

#[derive(Default, serde::Serialize)]
pub struct HealthCheckResponse {
    status: StatusEnum,
}

impl HealthCheckResponse {
    pub const fn up() -> Self {
        Self {
            status: StatusEnum::Up,
        }
    }
}
