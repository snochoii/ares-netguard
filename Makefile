PYTHON ?= $(shell if [ -x .venv/bin/python ]; then echo .venv/bin/python; else echo python3; fi)
CARGO ?= $(shell if [ -x $$HOME/.cargo/bin/cargo ]; then echo $$HOME/.cargo/bin/cargo; else echo cargo; fi)
PYTHONPATH := src$(if $(PYTHONPATH),:$(PYTHONPATH))
MPLCONFIGDIR ?= /tmp/ares-netguard-matplotlib
export PYTHONPATH
export MPLCONFIGDIR

FOUNDATION_PYTHON ?= $(PYTHON)
CHRONOS_MODEL_ROOT ?=
B1_ISOLATED_ROOT ?=
B1_WHEELHOUSE ?=
B1_INSTALL_REPORT ?=

.PHONY: verify verify-codex-workflow verify-rust-core fixture-smoke verify-foundation-forecast

verify: verify-codex-workflow
	$(PYTHON) -m ruff check .
	$(PYTHON) -m ruff format --check .
	$(PYTHON) -m compileall -q src tests
	$(PYTHON) -m pytest -q
	git diff --check
	git diff --cached --check
	bash scripts/check_no_generated_artifacts.sh --tracked
	bash scripts/check_no_generated_artifacts.sh --staged

verify-codex-workflow:
	$(PYTHON) scripts/validate_codex_workflow.py all
	$(PYTHON) -m pytest -q tests/unit/test_codex_workflow_contract.py

verify-rust-core:
	cd apps/rust-core && $(CARGO) fmt --check
	cd apps/rust-core && $(CARGO) test
	cd apps/rust-core && $(CARGO) clippy -- -D warnings

verify-foundation-forecast:
	test -n "$(B1_ISOLATED_ROOT)" || (echo "B1_ISOLATED_ROOT is required" >&2; exit 2)
	test -n "$(B1_WHEELHOUSE)" || (echo "B1_WHEELHOUSE is required" >&2; exit 2)
	test -n "$(B1_INSTALL_REPORT)" || (echo "B1_INSTALL_REPORT is required" >&2; exit 2)
	test -n "$(CHRONOS_MODEL_ROOT)" || (echo "CHRONOS_MODEL_ROOT is required" >&2; exit 2)
	test "$$(realpath -m "$(B1_ISOLATED_ROOT)")" != "/tmp" || \
		(echo "B1_ISOLATED_ROOT must be a bounded child of /tmp" >&2; exit 2)
	test "$$(realpath -m "$(B1_ISOLATED_ROOT)")" = \
		"$$(realpath -m "$(B1_ISOLATED_ROOT)" | sed -n 's#^\(/tmp/ares-netguard-b1\.[A-Za-z0-9]*\)$$#\1#p')" || \
		(echo "B1_ISOLATED_ROOT must match /tmp/ares-netguard-b1.XXXXXX" >&2; exit 2)
	mkdir -p "$(B1_ISOLATED_ROOT)/reports"
	$(FOUNDATION_PYTHON) scripts/verify_foundation_forecast_environment.py \
		--repo-root . \
		--isolation-root "$(B1_ISOLATED_ROOT)" \
		--wheelhouse "$(B1_WHEELHOUSE)" \
		--install-report "$(B1_INSTALL_REPORT)" \
		--model-root "$(CHRONOS_MODEL_ROOT)" \
		--output "$(B1_ISOLATED_ROOT)/environment-attestation.json"
	HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1 DO_NOT_TRACK=1 \
		$(FOUNDATION_PYTHON) -m ares_netguard.models.time_series_foundation_smoke \
		tests/fixtures/time_series_forecast/synthetic_windows.jsonl \
		tests/fixtures/time_series_forecast/anomaly_labels.jsonl \
		"$(CHRONOS_MODEL_ROOT)" \
		"$(B1_ISOLATED_ROOT)/reports" \
		--replay-windows tests/fixtures/time_series_forecast/replay_windows.jsonl \
		--replay-labels tests/fixtures/time_series_forecast/replay_anomaly_labels.jsonl \
		--environment-attestation "$(B1_ISOLATED_ROOT)/environment-attestation.json"
	$(FOUNDATION_PYTHON) -m json.tool \
		"$(B1_ISOLATED_ROOT)/reports/forecast-evaluation.json" >/dev/null
	$(FOUNDATION_PYTHON) -m json.tool \
		"$(B1_ISOLATED_ROOT)/reports/replay-evaluation.json" >/dev/null
	$(FOUNDATION_PYTHON) -m json.tool \
		"$(B1_ISOLATED_ROOT)/reports/operational-evidence.json" >/dev/null

fixture-smoke:
	mkdir -p /tmp/ares-netguard
	$(PYTHON) -m ares_netguard.ingest.telemetry_foundation \
		tests/fixtures/telemetry_foundation/synthetic_events.jsonl \
		/tmp/ares-netguard/telemetry-feature-windows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/telemetry-feature-windows.json >/dev/null
	$(PYTHON) -m ares_netguard.ingest.telemetry_foundation \
		tests/fixtures/detector_zoo/synthetic_events.jsonl \
		/tmp/ares-netguard/detector-zoo-feature-windows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/detector-zoo-feature-windows.json >/dev/null
	$(PYTHON) -m ares_netguard.models.detector_zoo \
		/tmp/ares-netguard/detector-zoo-feature-windows.json \
		/tmp/ares-netguard/detector-zoo-score-rows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/detector-zoo-score-rows.json >/dev/null
	$(PYTHON) -m ares_netguard.models.time_series_residual \
		tests/fixtures/time_series_residual/synthetic_windows.jsonl \
		/tmp/ares-netguard/time-series-residual-report.json \
		--backend rolling_mean_proxy \
		--history-window 3 \
		--calibration-window 8
	$(PYTHON) -m json.tool /tmp/ares-netguard/time-series-residual-report.json >/dev/null
	$(PYTHON) -m ares_netguard.models.self_supervised_representation \
		tests/fixtures/self_supervised_representation/synthetic_sequences.jsonl \
		/tmp/ares-netguard/traffic-representation-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/traffic-representation-report.json >/dev/null
	$(PYTHON) -m ares_netguard.graph.temporal_security_graph \
		tests/fixtures/temporal_security_graph/synthetic_edges.jsonl \
		/tmp/ares-netguard/temporal-security-graph-report.json \
		--history-window 3 \
		--min-history-windows 2
	$(PYTHON) -m json.tool /tmp/ares-netguard/temporal-security-graph-report.json >/dev/null
	$(PYTHON) -m ares_netguard.native_inference.adapters \
		tests/fixtures/native_inference/manifest.json \
		tests/fixtures/native_inference/feature_rows.jsonl \
		/tmp/ares-netguard/native-inference-score-rows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/native-inference-score-rows.json >/dev/null
	$(PYTHON) -m ares_netguard.models.score_row_composer \
		/tmp/ares-netguard/composed-model-score-rows.json \
		--score-rows tests/fixtures/model_disagreement/synthetic_scores.jsonl \
		--score-rows /tmp/ares-netguard/detector-zoo-score-rows.json \
		--score-rows /tmp/ares-netguard/native-inference-score-rows.json \
		--residual-report /tmp/ares-netguard/time-series-residual-report.json \
		--representation-report /tmp/ares-netguard/traffic-representation-report.json \
		--graph-report /tmp/ares-netguard/temporal-security-graph-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/composed-model-score-rows.json >/dev/null
	$(PYTHON) -m ares_netguard.models.disagreement \
		/tmp/ares-netguard/composed-model-score-rows.json \
		/tmp/ares-netguard/model-disagreement-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/model-disagreement-report.json >/dev/null
	$(PYTHON) -m ares_netguard.investigation.agentic_layer \
		/tmp/ares-netguard/model-disagreement-report.json \
		/tmp/ares-netguard/agentic-investigation-report.json \
		--evidence-report /tmp/ares-netguard/time-series-residual-report.json \
		--evidence-report /tmp/ares-netguard/traffic-representation-report.json \
		--evidence-report /tmp/ares-netguard/temporal-security-graph-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/agentic-investigation-report.json >/dev/null
	$(PYTHON) -m ares_netguard.detection_engineering.candidates \
		/tmp/ares-netguard/model-disagreement-report.json \
		/tmp/ares-netguard/detection-candidate-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/detection-candidate-report.json >/dev/null
	$(PYTHON) -m ares_netguard.models.evaluation_bundle \
		/tmp/ares-netguard/model-evaluation-bundle.json \
		/tmp/ares-netguard/model-disagreement-report.json \
		/tmp/ares-netguard/time-series-residual-report.json \
		/tmp/ares-netguard/traffic-representation-report.json \
		/tmp/ares-netguard/temporal-security-graph-report.json \
		/tmp/ares-netguard/agentic-investigation-report.json \
		/tmp/ares-netguard/detection-candidate-report.json \
		/tmp/ares-netguard/composed-model-score-rows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/model-evaluation-bundle.json >/dev/null
	$(PYTHON) -m ares_netguard.models.registry_metadata \
		/tmp/ares-netguard/model-registry-metadata.json \
		/tmp/ares-netguard/model-evaluation-bundle.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/model-registry-metadata.json >/dev/null
	$(PYTHON) -m ares_netguard.storage.evidence_index \
		/tmp/ares-netguard/evidence-index.json \
		/tmp/ares-netguard/telemetry-feature-windows.json \
		/tmp/ares-netguard/detector-zoo-feature-windows.json \
		/tmp/ares-netguard/model-disagreement-report.json \
		/tmp/ares-netguard/time-series-residual-report.json \
		/tmp/ares-netguard/traffic-representation-report.json \
		/tmp/ares-netguard/temporal-security-graph-report.json \
		/tmp/ares-netguard/agentic-investigation-report.json \
		/tmp/ares-netguard/detection-candidate-report.json \
		/tmp/ares-netguard/composed-model-score-rows.json \
		/tmp/ares-netguard/model-registry-metadata.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/evidence-index.json >/dev/null
