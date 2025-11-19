_clear_terminal:
	@clear

# Rust commands

.PHONY: rust-static-analysis
rust-static-analysis: _clear_terminal
	@cargo fmt --all --check
	@cargo clippy --all-targets
	@cargo test --all-targets

.PHONY: rust-clean
rust-clean: _clear_terminal

.PHONY: rust-build
rust-build: _clear_terminal
	@cargo build

# Docker commands

.PHONY: docker-static-analysis
docker-static-analysis: _clear_terminal
	@dclint --disable-rule "no-unbound-port-interfaces" --disable-rule "require-quotes-in-ports" --disable-rule "require-project-name-field" --recursive .
	@hadolint services/client-application/Dockerfile
	@hadolint services/server-application/Dockerfile

.PHONY: docker-clean
docker-clean: _clear_terminal
	@docker compose rm -f

.PHONY: docker-start
docker-start: _clear_terminal
	@docker compose up --build --detach

.PHONY: docker-stop
docker-stop: _clear_terminal
	@docker compose down --remove-orphans --volumes

.PHONY: docker-logs
docker-logs: _clear_terminal
	@docker compose logs --follow client-application-1 client-application-2 client-application-3 server-application-1 server-application-2 server-application-3

.PHONY: docker-ps
docker-ps: _clear_terminal
	@while true; do \
		docker compose ps; \
		sleep 1; \
		clear; \
	done

# API commands

.PHONY: api-create-job-with-single-operation
api-create-job-with-single-operation: _clear_terminal
	@curl -X POST -H "Content-Type: text/plain" --data-binary @operations-single.txt "http://127.0.0.1:8080/api/jobs"

.PHONY: api-create-job-with-multiple-operations
api-create-job-with-multiple-operations: _clear_terminal
	@curl -X POST -H "Content-Type: text/plain" --data-binary @operations.txt "http://127.0.0.1:8080/api/jobs"

.PHONY: api-create-job-with-error-operation
api-create-job-with-error-operation: _clear_terminal
	@curl -X POST -H "Content-Type: text/plain" --data-binary @operations-error.txt "http://127.0.0.1:8080/api/jobs"

.PHONY: api-delete-job
JOB_ID ?= ""
api-delete-job: _clear_terminal
	@curl -X DELETE -H "Accept: application/json" "http://127.0.0.1:8080/api/jobs/$(JOB_ID)"

.PHONY: api-get-jobs
PAGE ?= 1
PAGE_SIZE ?= 100
api-get-jobs: _clear_terminal
	@curl -X GET -H "Accept: application/json" "http://127.0.0.1:8080/api/jobs?page=$(PAGE)&size=$(PAGE_SIZE)"

.PHONY: api-get-job
JOB_ID ?= ""
api-get-job: _clear_terminal
	@curl -X GET -H "Accept: application/json" "http://127.0.0.1:8080/api/jobs/$(JOB_ID)"

.PHONY: api-get-job-operations
JOB_ID ?= ""
PAGE ?= 1
PAGE_SIZE ?= 100
api-get-job-operations: _clear_terminal
	@curl -X GET -H "Accept: application/json" "http://127.0.0.1:8080/api/jobs/$(JOB_ID)/operations?page=$(PAGE)&size=$(PAGE_SIZE)"

.PHONY: api-get-job-operation
JOB_ID ?= ""
OPERATION_ID ?= ""
api-get-job-operation: _clear_terminal
		@curl -X GET -H "Accept: application/json" "http://127.0.0.1:8080/api/jobs/$(JOB_ID)/operations/$(OPERATION_ID)"
