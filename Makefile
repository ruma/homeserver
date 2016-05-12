test:
	cargo test
	docker stop ruma_test_postgres
	docker rm -v ruma_test_postgres
