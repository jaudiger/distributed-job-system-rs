use crate::domain;
use crate::domain::job::JobStatus;

// Job models

#[derive(serde::Serialize)]
pub struct NewJobResponse {
    id: String,
    created_operations: usize,
    status: JobStatus,
}

impl NewJobResponse {
    pub fn new(id: impl Into<String>, created_operations: usize) -> Self {
        Self {
            id: id.into(),
            created_operations,
            status: JobStatus::InProgress,
        }
    }
}

#[derive(serde::Serialize)]
pub struct JobResponse {
    id: String,
    operations: usize,
    status: JobStatus,
}

impl JobResponse {
    pub fn new(job: &domain::job::Job, total_completed_operations: usize) -> Self {
        Self {
            id: job.id(),
            operations: job.operations(),
            status: job.status(total_completed_operations),
        }
    }
}

#[derive(serde::Serialize)]
pub struct MinimalJobResponse {
    id: String,
}

impl From<&domain::job::Job> for MinimalJobResponse {
    fn from(job: &domain::job::Job) -> Self {
        Self { id: job.id() }
    }
}

// Operation models

#[derive(serde::Serialize)]
pub struct OperationResponse {
    id: String,
    request: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
}

impl From<domain::operation::Operation> for OperationResponse {
    fn from(operation: domain::operation::Operation) -> Self {
        Self {
            id: operation.id(),
            request: operation.request().to_string(),
            result: operation.result().map(ToString::to_string),
        }
    }
}

#[derive(serde::Serialize)]
pub struct MinimalOperationResponse {
    id: String,
}

impl From<&domain::operation::Operation> for MinimalOperationResponse {
    fn from(operation: &domain::operation::Operation) -> Self {
        Self { id: operation.id() }
    }
}

// Misc models

#[derive(Debug, serde::Deserialize)]
pub struct PageParams {
    page: Option<usize>,
    size: Option<usize>,
}

impl PageParams {
    const DEFAULT_PAGE: usize = 1;
    const DEFAULT_SIZE: usize = 30;

    const MIN_PAGE: usize = 1;
    const MIN_SIZE: usize = 1;
    const MAX_SIZE: usize = 100;

    pub fn page(&self) -> usize {
        self.page.unwrap_or(Self::DEFAULT_PAGE).max(Self::MIN_PAGE)
    }

    pub fn size(&self) -> usize {
        self.size
            .unwrap_or(Self::DEFAULT_SIZE)
            .clamp(Self::MIN_SIZE, Self::MAX_SIZE)
    }
}

#[derive(serde::Serialize)]
pub struct PageResponse<T> {
    page: usize,
    size: usize,
    total: usize,
    items: Vec<T>,
}

impl<T> PageResponse<T> {
    pub const fn new(page: usize, size: usize, total: usize, items: Vec<T>) -> Self {
        Self {
            page,
            size,
            total,
            items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageParams;

    #[test]
    fn page_defaults_when_missing() {
        // Arrange
        let params = PageParams {
            page: None,
            size: None,
        };

        // Act
        let page = params.page();

        // Assert
        assert_eq!(page, 1);
    }

    #[test]
    fn page_clamps_zero_to_minimum() {
        // Arrange
        let params = PageParams {
            page: Some(0),
            size: None,
        };

        // Act
        let page = params.page();

        // Assert
        assert_eq!(page, 1);
    }

    #[test]
    fn page_passes_through_in_range_value() {
        // Arrange
        let params = PageParams {
            page: Some(7),
            size: None,
        };

        // Act
        let page = params.page();

        // Assert
        assert_eq!(page, 7);
    }
}
