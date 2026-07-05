PYTHON ?= $(shell if [ -x .venv/bin/python ]; then echo .venv/bin/python; else echo python3; fi)
PYTHONPATH := src$(if $(PYTHONPATH),:$(PYTHONPATH))
export PYTHONPATH

.PHONY: verify fixture-smoke

verify:
	$(PYTHON) -m ruff check .
	$(PYTHON) -m ruff format --check .
	$(PYTHON) -m compileall -q src tests
	$(PYTHON) -m pytest -q
	git diff --check
	git diff --cached --check
	bash scripts/check_no_generated_artifacts.sh --tracked
	bash scripts/check_no_generated_artifacts.sh --staged

fixture-smoke:
	mkdir -p /tmp/ares-netguard
	$(PYTHON) -m ares_netguard.models.disagreement \
		tests/fixtures/model_disagreement/synthetic_scores.jsonl \
		/tmp/ares-netguard/model-disagreement-report.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/model-disagreement-report.json >/dev/null
	$(PYTHON) -m ares_netguard.models.time_series_residual \
		tests/fixtures/time_series_residual/synthetic_windows.jsonl \
		/tmp/ares-netguard/time-series-residual-report.json
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
	$(PYTHON) -m ares_netguard.native_inference.adapters \
		tests/fixtures/native_inference/manifest.json \
		tests/fixtures/native_inference/feature_rows.jsonl \
		/tmp/ares-netguard/native-inference-score-rows.json
	$(PYTHON) -m json.tool /tmp/ares-netguard/native-inference-score-rows.json >/dev/null
