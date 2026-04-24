"""AI Handlers for scheduled jobs"""

from .base_handler import BaseHandler
from .price_anomaly_handler import PriceAnomalyHandler
from .price_suggestion_handler import PriceSuggestionHandler
from .sales_anomaly_handler import SalesAnomalyHandler
from .demand_forecast_handler import DemandForecastHandler
from .classification_handler import ClassificationHandler
from .clustering_handler import ClusteringHandler
from .supplier_scoring_handler import SupplierScoringHandler

__all__ = [
    "BaseHandler",
    "PriceAnomalyHandler",
    "PriceSuggestionHandler",
    "SalesAnomalyHandler",
    "DemandForecastHandler",
    "ClassificationHandler",
    "ClusteringHandler",
    "SupplierScoringHandler",
]
