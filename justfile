set dotenv-load := true

install:
	cargo install --path .

staging-profiles:
	nexus staging profiles

staging-start:
	nexus staging start "Just a test"
