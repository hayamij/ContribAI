"""Tests for Phase 7: model registry, router, agents, multi-model config."""

from __future__ import annotations

import pytest

from contribai.core.config import (
    ContribAIConfig,
    MultiModelConfig,
)
from contribai.llm.agents import (
    AgentCoordinator,
    AgentResult,
    AnalysisAgent,
    BaseAgent,
    CodeGenAgent,
    DocsAgent,
    PlannerAgent,
    ReviewAgent,
)
from contribai.llm.models import (
    ALL_MODELS,
    GEMINI_2_5_FLASH,
    GEMINI_3_1_FLASH_LITE,
    GEMINI_3_1_PRO,
    GEMINI_3_FLASH,
    MODELS_BY_NAME,
    MODELS_BY_TIER,
    ModelSpec,
    ModelTier,
    TaskType,
    get_cheapest_capable,
    get_model,
    get_models_for_task,
)
from contribai.llm.router import (
    CostStrategy,
    RoutingDecision,
    TaskRouter,
)


# ── Config Tests ───────────────────────────────────


class TestMultiModelConfig:
    def test_defaults(self):
        cfg = MultiModelConfig()
        assert cfg.enabled is False
        assert cfg.strategy == "balanced"
        assert cfg.model_overrides == {}

    def test_full_config(self):
        cfg = ContribAIConfig()
        assert hasattr(cfg, "multi_model")
        assert isinstance(cfg.multi_model, MultiModelConfig)


# ── Model Registry Tests ──────────────────────────


class TestModelRegistry:
    def test_all_models_exist(self):
        assert len(ALL_MODELS) >= 5

    def test_model_tiers(self):
        assert GEMINI_3_1_PRO.tier == ModelTier.PRO
        assert GEMINI_3_FLASH.tier == ModelTier.FLASH
        assert GEMINI_3_1_FLASH_LITE.tier == ModelTier.LITE

    def test_model_by_name(self):
        m = get_model("gemini-3.1-pro-preview")
        assert m is not None
        assert m.display_name == "Gemini 3.1 Pro"

    def test_model_not_found(self):
        assert get_model("nope") is None

    def test_models_by_tier(self):
        pros = MODELS_BY_TIER[ModelTier.PRO]
        assert len(pros) >= 2

    def test_pro_highest_coding(self):
        assert GEMINI_3_1_PRO.coding >= 95
        assert GEMINI_3_1_PRO.coding > GEMINI_3_FLASH.coding

    def test_lite_cheapest(self):
        assert GEMINI_3_1_FLASH_LITE.input_cost < GEMINI_3_FLASH.input_cost
        assert GEMINI_3_1_FLASH_LITE.input_cost < GEMINI_3_1_PRO.input_cost

    def test_overall_score(self):
        score = GEMINI_3_1_PRO.overall_score
        assert 80 <= score <= 100

    def test_cost_efficiency(self):
        lite_eff = GEMINI_3_1_FLASH_LITE.cost_efficiency
        pro_eff = GEMINI_3_1_PRO.cost_efficiency
        assert lite_eff > pro_eff  # Lite is more cost-efficient

    def test_models_for_task_codegen(self):
        models = get_models_for_task(TaskType.CODE_GEN)
        assert len(models) >= 2
        # Pro should be top for coding
        assert models[0].coding >= 88

    def test_models_for_task_bulk(self):
        models = get_models_for_task(TaskType.BULK)
        assert len(models) >= 1
        # Should include lite
        assert any(m.tier == ModelTier.LITE for m in models)

    def test_cheapest_capable(self):
        m = get_cheapest_capable(TaskType.ANALYSIS, min_score=70)
        assert m is not None
        assert m.input_cost <= GEMINI_3_1_PRO.input_cost

    def test_model_best_for(self):
        assert TaskType.CODE_GEN in GEMINI_3_1_PRO.best_for
        assert TaskType.BULK in GEMINI_3_1_FLASH_LITE.best_for


# ── Router Tests ─────────────────────────────────


class TestTaskRouter:
    def test_balanced_high_complexity(self):
        router = TaskRouter(strategy=CostStrategy.BALANCED)
        decision = router.route(TaskType.ANALYSIS, complexity=9, file_count=15)
        assert decision.model.tier == ModelTier.PRO
        assert decision.fallback is not None

    def test_balanced_low_complexity(self):
        router = TaskRouter(strategy=CostStrategy.BALANCED)
        decision = router.route(TaskType.QUICK_FIX, complexity=2)
        assert decision.model.tier == ModelTier.LITE

    def test_balanced_medium(self):
        router = TaskRouter(strategy=CostStrategy.BALANCED)
        decision = router.route(TaskType.ANALYSIS, complexity=5)
        assert decision.model.tier == ModelTier.FLASH

    def test_performance_mode(self):
        router = TaskRouter(strategy=CostStrategy.PERFORMANCE)
        decision = router.route(TaskType.CODE_GEN)
        assert decision.model.coding >= 88

    def test_economy_mode(self):
        router = TaskRouter(strategy=CostStrategy.ECONOMY)
        decision = router.route(TaskType.DOCS)
        assert decision.model.input_cost <= 0.20

    def test_planning_always_pro(self):
        router = TaskRouter(strategy=CostStrategy.BALANCED)
        decision = router.route(TaskType.PLANNING, complexity=3)
        assert decision.model.tier == ModelTier.PRO

    def test_bulk_always_lite(self):
        router = TaskRouter(strategy=CostStrategy.BALANCED)
        decision = router.route(TaskType.BULK, complexity=3)
        assert decision.model.tier == ModelTier.LITE

    def test_routing_stats(self):
        router = TaskRouter()
        router.route(TaskType.ANALYSIS, complexity=5)
        router.route(TaskType.CODE_GEN, complexity=5)
        stats = router.stats
        assert stats["total_tasks"] == 2

    def test_default_assignments(self):
        router = TaskRouter()
        defaults = router.get_default_assignments()
        assert TaskType.PLANNING in defaults
        assert TaskType.BULK in defaults

    def test_routing_decision_fields(self):
        decision = RoutingDecision(
            model=GEMINI_3_FLASH,
            task_type=TaskType.ANALYSIS,
            reason="test",
        )
        assert decision.model == GEMINI_3_FLASH
        assert decision.fallback is None


# ── Agent Tests ──────────────────────────────────


class TestAgents:
    def test_agent_result(self):
        r = AgentResult(
            agent_name="test",
            model_used="gemini-3-flash",
            task_type="analysis",
            output="found issues",
            success=True,
        )
        assert r.success
        assert r.error == ""

    def test_analysis_agent_prompt(self):
        router = TaskRouter()
        agent = AnalysisAgent(None, router)
        prompt = agent.build_prompt(
            {
                "code": "def foo(): pass",
                "language": "python",
                "file_path": "test.py",
            }
        )
        assert "python" in prompt
        assert "test.py" in prompt

    def test_codegen_agent_prompt(self):
        router = TaskRouter()
        agent = CodeGenAgent(None, router)
        prompt = agent.build_prompt(
            {
                "issue": "SQL injection",
                "original_code": "SELECT * FROM users",
                "language": "python",
            }
        )
        assert "SQL injection" in prompt

    def test_review_agent_prompt(self):
        router = TaskRouter()
        agent = ReviewAgent(None, router)
        prompt = agent.build_prompt(
            {
                "original_code": "old",
                "modified_code": "new",
                "issue": "fix bug",
            }
        )
        assert "fix bug" in prompt
        assert "old" in prompt
        assert "new" in prompt

    def test_docs_agent_prompt(self):
        router = TaskRouter()
        agent = DocsAgent(None, router)
        prompt = agent.build_prompt({"code": "def foo(): pass", "doc_type": "docstring"})
        assert "docstring" in prompt

    def test_planner_agent_prompt(self):
        router = TaskRouter()
        agent = PlannerAgent(None, router)
        prompt = agent.build_prompt({"repo_name": "owner/repo", "findings": "issues"})
        assert "owner/repo" in prompt

    def test_agent_system_prompts(self):
        router = TaskRouter()
        agents = [
            AnalysisAgent(None, router),
            CodeGenAgent(None, router),
            ReviewAgent(None, router),
            DocsAgent(None, router),
            PlannerAgent(None, router),
        ]
        for agent in agents:
            assert len(agent.system_prompt()) > 20

    def test_coordinator_creation(self):
        coord = AgentCoordinator(
            llm_provider=None,
            strategy=CostStrategy.BALANCED,
        )
        assert "analyzer" in coord._agents
        assert "codegen" in coord._agents
        assert "reviewer" in coord._agents
        assert "docs" in coord._agents
        assert "planner" in coord._agents

    def test_coordinator_routing_stats(self):
        coord = AgentCoordinator(None)
        stats = coord.routing_stats
        assert "strategy" in stats
        assert stats["total_tasks"] == 0


# ── Version Tests ────────────────────────────────


class TestVersion:
    def test_version(self):
        from contribai import __version__

        assert __version__ == "0.10.0"
