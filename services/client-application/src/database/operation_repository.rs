use crate::application::counter;
use crate::database;
use crate::domain;
use anyhow::Result;
use futures::Future;
use futures::TryStreamExt;
use mongodb::Collection;
use mongodb::IndexModel;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;

counter!(
    INSERT_OPERATION_COUNTER,
    "database_insert_operation_requests",
    "Number of insert operation requests"
);
counter!(
    INSERT_OPERATIONS_COUNTER,
    "database_insert_operations_requests",
    "Number of insert operations requests"
);
counter!(
    DELETE_OPERATIONS_COUNTER,
    "database_delete_operations_requests",
    "Number of delete operations requests"
);
counter!(
    GET_OPERATION_COUNTER,
    "database_get_operation_requests",
    "Number of get operation requests"
);
counter!(
    GET_TOTAL_COMPLETED_OPERATIONS_COUNTER,
    "database_get_total_completed_operations_requests",
    "Number of get total completed operations requests"
);
counter!(
    GET_OPERATIONS_COUNTER,
    "database_get_operations_requests",
    "Number of get operations requests"
);
counter!(
    GET_BATCH_OPERATIONS_COUNTER,
    "database_get_batch_operations_requests",
    "Number of get batch operations requests"
);
counter!(
    UPDATE_OPERATION_COUNTER,
    "database_update_operation_requests",
    "Number of update operation requests"
);

pub struct OperationRepository {
    collection: Collection<domain::operation::Operation>,
}

impl OperationRepository {
    pub const COLLECTION_NAME: &'static str = "operation";

    const ID_FIELD: &'static str = "_id";
    const JOB_ID_FIELD: &'static str = "job_id";
    const RESULT_FIELD: &'static str = "result";

    pub async fn new(collection: Collection<domain::operation::Operation>) -> Result<Self> {
        tracing::debug!("Initializing the MongoDB operation repository");

        let job_id_index = IndexModel::builder()
            .keys(doc! { Self::JOB_ID_FIELD: 1 })
            .build();
        let _ = collection.create_index(job_id_index).await?;

        let result_index = IndexModel::builder()
            .keys(doc! { Self::RESULT_FIELD: 1 })
            .build();
        let _ = collection.create_index(result_index).await?;

        Ok(Self { collection })
    }

    #[allow(unused)]
    #[tracing::instrument(skip(self))]
    pub async fn insert_operation(
        &self,
        new_operation: &domain::operation::Operation,
    ) -> Result<String> {
        tracing::debug!("Inserting an operation");

        INSERT_OPERATION_COUNTER.add(1, &[]);

        let result = self.collection.insert_one(new_operation).await?;

        Ok(result
            .inserted_id
            .as_object_id()
            .expect("No ObjectId returned")
            .to_string())
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_operations(
        &self,
        new_operation: &Vec<domain::operation::Operation>,
    ) -> Result<()> {
        tracing::debug!("Inserting operations");

        INSERT_OPERATIONS_COUNTER.add(1, &[]);

        let _ = self.collection.insert_many(new_operation).await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_operations(&self, job_id: impl AsRef<str> + std::fmt::Debug) -> Result<()> {
        tracing::debug!("Deleting operations of job {}", job_id.as_ref());

        DELETE_OPERATIONS_COUNTER.add(1, &[]);

        let _ = self
            .collection
            .delete_many(doc! {Self::JOB_ID_FIELD: job_id.as_ref()})
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_operation(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
        operation_id: impl AsRef<str> + std::fmt::Debug,
    ) -> Result<domain::operation::Operation> {
        tracing::debug!(
            "Getting operation {} for job {}",
            operation_id.as_ref(),
            job_id.as_ref()
        );

        GET_OPERATION_COUNTER.add(1, &[]);

        let result = self
            .collection
            .find_one(doc! {
                Self::ID_FIELD: ObjectId::parse_str(operation_id)?,
                Self::JOB_ID_FIELD: job_id.as_ref()
            })
            .await?;

        if let Some(result) = result {
            Ok(result)
        } else {
            anyhow::bail!("Document not found");
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_total_completed_operations(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
    ) -> Result<usize> {
        tracing::debug!(
            "Getting total completed operations for job {}",
            job_id.as_ref()
        );

        GET_TOTAL_COMPLETED_OPERATIONS_COUNTER.add(1, &[]);

        let result = self
            .collection
            .count_documents(doc! {
                Self::JOB_ID_FIELD: job_id.as_ref(),
                Self::RESULT_FIELD: { "$exists": true, "$ne": "" }
            })
            .await?;

        usize::try_from(result).map_err(|err| anyhow::anyhow!(err))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_operations(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
        page: usize,
        page_size: usize,
    ) -> Result<database::model::PageSubset<domain::operation::Operation>> {
        tracing::debug!("Getting operations for job {}", job_id.as_ref());

        GET_OPERATIONS_COUNTER.add(1, &[]);

        let skip = ((page - 1) * page_size) as u64;
        let filter = doc! {Self::JOB_ID_FIELD: job_id.as_ref()};

        #[allow(clippy::cast_possible_wrap)]
        let mut cursor = self
            .collection
            .find(filter.clone())
            .limit(page_size as i64)
            .skip(skip)
            .await?;

        let mut operations = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            operations.push(doc);
        }

        let total = self
            .collection
            .count_documents(filter)
            .await
            .map(usize::try_from)??;

        Ok(database::model::PageSubset::new(total, operations))
    }

    #[tracing::instrument(skip(self, handler))]
    pub async fn get_batch_operations<F, Fut>(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
        batch_size: u32,
        mut handler: F,
    ) -> Result<()>
    where
        F: FnMut(domain::operation::Operation) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        tracing::debug!("Getting operations for job {}", job_id.as_ref());

        GET_BATCH_OPERATIONS_COUNTER.add(1, &[]);

        let cursor = self
            .collection
            .find(doc! { Self::JOB_ID_FIELD: job_id.as_ref() })
            .batch_size(batch_size)
            .await?;

        let mut chunked = cursor.try_chunks(batch_size as usize);

        while let Some(batch) = chunked.try_next().await? {
            tracing::trace!("Processing a chunk of {} operations", batch.len());

            for op in batch {
                handler(op).await;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_operation(
        &self,
        job_id: impl AsRef<str> + std::fmt::Debug,
        operation_id: impl AsRef<str> + std::fmt::Debug,
        result: impl AsRef<str> + std::fmt::Debug,
    ) -> Result<()> {
        tracing::debug!(
            "Updating operation {} for job {}",
            operation_id.as_ref(),
            job_id.as_ref()
        );

        UPDATE_OPERATION_COUNTER.add(1, &[]);

        let result = self
            .collection
            .update_one(
                doc! {
                    Self::ID_FIELD: ObjectId::parse_str(operation_id)?,
                    Self::JOB_ID_FIELD: job_id.as_ref()
                },
                doc! {
                    "$set": doc! {Self::RESULT_FIELD: Some(result.as_ref())}
                },
            )
            .await?;

        if result.matched_count == 0 {
            anyhow::bail!("Document not found");
        }

        Ok(())
    }
}
