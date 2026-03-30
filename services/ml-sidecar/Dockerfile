# Investor OS — ML Sidecar (CPU)
# Python FastAPI service for ML model inference
#
# Build:  docker build -t ios-ml-sidecar .
# Run:    docker run -p 9000:9000 ios-ml-sidecar

FROM python:3.11-slim AS base

WORKDIR /app

# System deps for numpy/pandas/scikit-learn (needed later for catboost etc.)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Python deps
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy app
COPY . .

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD curl -f http://localhost:9000/health || exit 1

EXPOSE 9000

CMD ["python", "-m", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "9000", "--workers", "1"]
