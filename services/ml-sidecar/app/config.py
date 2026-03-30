"""ML Sidecar configuration via environment variables."""

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    """Application settings loaded from environment."""

    # Service
    app_name: str = "ios-ml-sidecar"
    app_version: str = "0.1.0"
    log_level: str = "info"

    # Database
    database_url: str = "postgresql+asyncpg://investor:investor@postgres:5432/investor_os"

    # Redis
    redis_url: str = "redis://redis:6379"

    # Model storage
    model_dir: str = "/app/models"

    # Inference
    default_timeout_ms: int = 5000
    max_batch_size: int = 16

    model_config = {"env_prefix": "ML_", "case_sensitive": False}


settings = Settings()
