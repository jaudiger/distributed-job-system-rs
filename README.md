# Distributed Job System

This project consists of two Rust applications (client and server) that work together with MongoDB, Kafka, Prometheus, and Jaeger to provide a distributed job system. This system processes mathematical operations that are sent to a HTTP endpoint.

## High-Level Architecture

The system consists of:

1. Client Application (3 instances):
   - HTTP server exposing job management API
   - Produces messages to Kafka
   - Connects to MongoDB for job and operation storage
   - Instruments with OpenTelemetry for tracing (Jaeger) and metrics (Prometheus)

2. Server Application (3 instances):
   - Consumes messages from Kafka
   - Processes operations
   - Instruments with OpenTelemetry

3. MongoDB (3 instances):
   - Primary-replica architecture
   - Stores job and operation data

4. Kafka (3 instances):
   - Distributed message queue
   - Handles communication between client and server applications

5. Prometheus:
   - Metrics collection and monitoring

6. Jaeger:
   - Distributed tracing

This architecture leverages Rust's performance and safety features while providing a scalable, fault-tolerant system for job processing. All the services are scaled to 3 instances for high availability, and the Kafka consumers' concurrency is set between 10 and 20 to take advantage of multi-core systems. Kafka topics contains 60 partitions for the topic to ensure good distribution of messages across the cluster. Here, when all the Kafka consumers are up, we have at most 20 threads per instance, and since we have 3 instances, we can assign one partition to one thread.

## Getting Started

### Tools Needed

- Rust
- Docker and Docker Compose
- Make
- cURL (for API testing)

### Running the Project

1. Build and start all services: `make docker-start`. It can take up to 1 minute to boot everything, since the database and the messaging layers will be initialized with sane defaults.
2. View logs: `make docker-logs`

### API endpoints

When the system is running, you can:

1. Create a job: `make api-create-job-with-single-operation` or `make api-create-job-with-multiple-operations` or `make api-create-job-with-error-operation`
2. List all jobs: `make api-get-jobs`
3. Get a specific job: `make api-get-job JOB_ID=<job_id>`
4. List operations for a job: `make api-get-job-operations JOB_ID=<job_id>`
5. Get a specific operation: `make api-get-job-operation JOB_ID=<job_id> OPERATION_ID=<operation_id>`

### Stopping the Project

Just call: `make docker-stop`

## Observability

### Logs

Services logs configured to be printed to stdout and stderr, so you can see them in the terminal.

### Metrics

By default, metrics are sent every 30s, to change this, you can set the `OTEL_METRIC_EXPORT_INTERVAL` environment variable in the `docker-compose.yml` file. The value is expressed in milliseconds.

Metrics are exported to Prometheus, which is accessible on port `9090`. You can view them by navigating to `http://localhost:9090/query`.

To see the number of messages processed by the `server-application`, you can navigate to `http://localhost:9090/query?g0.expr=consumer_messages_received_total%7Btopic%3D%22application.operation.request%22%7D`. And to see the number of messages processed by the `client-application`, you can navigate to `http://localhost:9090/query?g0.expr=consumer_messages_received_total%7Btopic%3D%22application.operation.response%22%7D`.

### Traces

By default, traces sampling are disabled, since the more traces you sample, the more resources it'll take to process workloads, which can have a negative impact when handling large requests from the API. To enable, the traces, you can set the `OTEL_TRACES_SAMPLER`  in the docker-compose file to `parentbased_traceidratio` and set the `OTEL_TRACES_SAMPLER_ARG` to the desired sampling rate. For example, to sample `10%` of the traces, you can set the `OTEL_TRACES_SAMPLER_ARG` to `0.1`. Setting the `OTEL_TRACES_SAMPLER_ARG` to `1` will sample all traces, but this is not recommended for production environments.

Traces are exported to Jaeger, which is accessible on port `16686`. You can view them by navigating to `http://localhost:16686`.

To see the traces when creating a new job, you can navigate to: `http://127.0.0.1:16686/search?service=client-application`
