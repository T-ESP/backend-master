FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Cache HuggingFace downloads inside the image's volume so a re-pull doesn't
# re-download embedding weights.
ENV HF_HOME=/app/embed_cache \
    SENTENCE_TRANSFORMERS_HOME=/app/embed_cache \
    PIP_NO_CACHE_DIR=1

COPY requirements.txt .

# Installer torch CPU-only AVANT le reste : sinon sentence-transformers tire la
# version CUDA de torch (~2 GB de libs nvidia inutiles sur un VPS sans GPU).
# Pré-installer la variante CPU fait que pip la considère déjà satisfaite.
RUN pip install --no-cache-dir torch --index-url https://download.pytorch.org/whl/cpu

RUN pip install --no-cache-dir -r requirements.txt

COPY . .

CMD ["python", "main.py"]
