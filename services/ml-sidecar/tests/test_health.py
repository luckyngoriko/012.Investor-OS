"""Tests for ML Sidecar health endpoints."""

from fastapi.testclient import TestClient

from app.main import app

client = TestClient(app)


def test_health_returns_200():
    response = client.get("/health")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "healthy"
    assert data["version"] == "0.1.0"
    assert isinstance(data["models_loaded"], list)
    assert data["gpu_available"] is False
    assert data["uptime_seconds"] >= 0


def test_ready_returns_200():
    response = client.get("/ready")
    assert response.status_code == 200
    data = response.json()
    assert data["ready"] is True


def test_info_returns_endpoints():
    response = client.get("/info")
    assert response.status_code == 200
    data = response.json()
    assert data["service"] == "ios-ml-sidecar"
    assert data["tier"] == 1
    assert "predict" in data["endpoints"]
    assert "optimize" in data["endpoints"]
