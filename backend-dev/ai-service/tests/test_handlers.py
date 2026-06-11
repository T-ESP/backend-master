"""Tests for AI service handlers."""
import unittest
from unittest.mock import Mock, patch, MagicMock
from handlers.price_suggestion_handler import PriceSuggestionHandler
from handlers.price_anomaly_handler import PriceAnomalyHandler
from handlers.sales_anomaly_handler import SalesAnomalyHandler
from handlers.demand_forecast_handler import DemandForecastHandler
from handlers.clustering_handler import ClusteringHandler
from handlers.supplier_scoring_handler import SupplierScoringHandler
from handlers.classification_handler import ClassificationHandler


class TestHandlerInstantiation(unittest.TestCase):
    """Test that all handlers can be instantiated."""

    def test_price_suggestion_handler(self):
        handler = PriceSuggestionHandler()
        self.assertIsNotNone(handler)

    def test_price_anomaly_handler(self):
        handler = PriceAnomalyHandler()
        self.assertIsNotNone(handler)

    def test_sales_anomaly_handler(self):
        handler = SalesAnomalyHandler()
        self.assertIsNotNone(handler)

    def test_demand_forecast_handler(self):
        handler = DemandForecastHandler()
        self.assertIsNotNone(handler)

    def test_clustering_handler(self):
        handler = ClusteringHandler()
        self.assertIsNotNone(handler)

    def test_supplier_scoring_handler(self):
        handler = SupplierScoringHandler()
        self.assertIsNotNone(handler)

    def test_classification_handler(self):
        handler = ClassificationHandler()
        self.assertIsNotNone(handler)


class TestHandlerErrorResilience(unittest.TestCase):
    """Test that handlers handle database errors gracefully."""

    @patch('handlers.demand_forecast_handler.get_connection')
    @patch('handlers.demand_forecast_handler.return_connection')
    def test_demand_forecast_handles_empty_data(self, mock_return, mock_get):
        """DemandForecastHandler calls model.forecast_all_products(conn, ...) which returns results.
        When results are empty, the handler should still return connection."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        handler = DemandForecastHandler()
        # Mock the model to return an empty list of results
        handler.model = MagicMock()
        handler.model.forecast_all_products.return_value = []
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.clustering_handler.get_connection')
    @patch('handlers.clustering_handler.return_connection')
    def test_clustering_handles_empty_data(self, mock_return, mock_get):
        """ClusteringHandler calls model.cluster_all_products(conn, ...).
        When result has an error, the handler should still return connection."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        handler = ClusteringHandler()
        # Mock the model to return an error result
        handler.model = MagicMock()
        handler.model.cluster_all_products.return_value = {"error": "Not enough data"}
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.price_suggestion_handler.get_connection')
    @patch('handlers.price_suggestion_handler.return_connection')
    @patch('handlers.price_suggestion_handler.get_products_with_prices')
    def test_price_suggestion_handles_no_products(self, mock_products, mock_return, mock_get):
        """PriceSuggestionHandler should handle empty product list gracefully."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        mock_products.return_value = []
        handler = PriceSuggestionHandler()
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.price_anomaly_handler.get_connection')
    @patch('handlers.price_anomaly_handler.return_connection')
    @patch('handlers.price_anomaly_handler.get_price_history')
    def test_price_anomaly_handles_no_data(self, mock_prices, mock_return, mock_get):
        """PriceAnomalyHandler should handle empty price history gracefully."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        mock_prices.return_value = []
        handler = PriceAnomalyHandler()
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.sales_anomaly_handler.get_connection')
    @patch('handlers.sales_anomaly_handler.return_connection')
    @patch('handlers.sales_anomaly_handler.get_sales_data')
    def test_sales_anomaly_handles_no_data(self, mock_sales, mock_return, mock_get):
        """SalesAnomalyHandler should handle empty sales data gracefully."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        mock_sales.return_value = []
        handler = SalesAnomalyHandler()
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.supplier_scoring_handler.get_connection')
    @patch('handlers.supplier_scoring_handler.return_connection')
    def test_supplier_scoring_handles_no_data(self, mock_return, mock_get):
        """SupplierScoringHandler should handle no-data result gracefully."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        handler = SupplierScoringHandler()
        handler.model = MagicMock()
        handler.model.score_all_suppliers.return_value = {"status": "no_data"}
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)

    @patch('handlers.classification_handler.get_connection')
    @patch('handlers.classification_handler.return_connection')
    def test_classification_handles_no_data(self, mock_return, mock_get):
        """ClassificationHandler should handle no-data result gracefully."""
        mock_conn = MagicMock()
        mock_get.return_value = mock_conn
        handler = ClassificationHandler()
        handler.model = MagicMock()
        handler.model.classify_all_products.return_value = {"status": "no_data"}
        result = handler.run()
        mock_return.assert_called_once_with(mock_conn)


if __name__ == '__main__':
    unittest.main()
