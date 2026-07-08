"""Chatbot module for ai-service.

Adds conversational AI on top of the existing batch ML service:
- LLM provider abstraction (Groq primary, Mistral fallback)
- RAG over project documentation (pgvector + multilingual MiniLM)
- Tool-use agent that wraps the existing stocks_api endpoints
- Action-confirmation gate for write operations

This module is stateless about chat sessions — Rust owns persistence.
Each /chat/turn call carries the full needed context.
"""
