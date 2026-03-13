"""
Unit Tests for AI Models

Tests for all AI models:
- Demand forecasting
- ABC-XYZ classification
- Product clustering
- Supplier scoring
- Price anomaly detection
- Sales anomaly detection
- Price suggestions
"""

import unittest
from unittest.mock import Mock, MagicMock, patch
from datetime import datetime, timedelta
import pandas as pd
import numpy as np

# Import models
from models.demand_forecaster import DemandForecaster
from models.abc_xyz_classifier import ABCXYZClassifier
from models.product_clusterer import ProductClusterer
from models.supplier_scorer import SupplierScorer
from models.price_anomaly_detector import PriceAnomalyDetector
from models.sales_anomaly_detector import SalesAnomalyDetector
from models.price_suggester import PriceSuggester


class TestDemandForecaster(unittest.TestCase):
    """Tests for DemandForecaster model"""

    def setUp(self):
        self.forecaster = DemandForecaster(model_dir="/tmp/test_models")

    def test_mape_calculation(self):
        """Test MAPE (Mean Absolute Percentage Error) calculation"""
        actual = np.array([100, 200, 150, 180])
        predicted = np.array([110, 190, 160, 170])

        mape = self.forecaster._calculate_mape(actual, predicted)

        # Expected: mean of |10/100|, |10/200|, |10/150|, |10/180| * 100
        self.assertGreater(mape, 0)
        self.assertLess(mape, 20)  # Should be reasonable error

    def test_mape_with_zeros(self):
        """Test MAPE handles zero values correctly"""
        actual = np.array([0, 100, 0, 200])
        predicted = np.array([10, 110, 10, 190])

        mape = self.forecaster._calculate_mape(actual, predicted)

        # Should only calculate for non-zero actual values
        self.assertIsInstance(mape, float)

    def test_rmse_calculation(self):
        """Test RMSE (Root Mean Square Error) calculation"""
        actual = np.array([100, 200, 150, 180])
        predicted = np.array([110, 190, 160, 170])

        rmse = self.forecaster._calculate_rmse(actual, predicted)

        # RMSE should be sqrt(mean((a-p)^2))
        expected = np.sqrt(np.mean((actual - predicted) ** 2))
        self.assertAlmostEqual(rmse, expected, places=4)


class TestABCXYZClassifier(unittest.TestCase):
    """Tests for ABCXYZClassifier model"""

    def setUp(self):
        self.classifier = ABCXYZClassifier()

    def test_abc_classification(self):
        """Test ABC classification based on revenue"""
        # Create test data with clear revenue tiers
        data = {
            'product_id': [1, 2, 3, 4, 5],
            'product_name': ['P1', 'P2', 'P3', 'P4', 'P5'],
            'total_revenue': [80000, 10000, 5000, 3000, 2000],  # 80%, 10%, 5%, 3%, 2%
            'total_units_sold': [100, 50, 30, 20, 10],
            'num_transactions': [10, 5, 3, 2, 1],
            'cv': [0.3, 0.6, 1.2, 0.4, 0.8]
        }
        df = pd.DataFrame(data)

        result = self.classifier._calculate_abc(df)

        # Product 1 should be A (80% cumulative)
        self.assertEqual(result[result['product_id'] == 1]['abc_class'].values[0], 'A')
        # Product 5 should be C (low revenue)
        self.assertEqual(result[result['product_id'] == 5]['abc_class'].values[0], 'C')

    def test_xyz_classification(self):
        """Test XYZ classification based on coefficient of variation"""
        data = {
            'product_id': [1, 2, 3],
            'cv': [0.3, 0.7, 1.5]  # X, Y, Z
        }
        df = pd.DataFrame(data)

        result = self.classifier._calculate_xyz(df)

        self.assertEqual(result[result['product_id'] == 1]['xyz_class'].values[0], 'X')  # CV < 0.5
        self.assertEqual(result[result['product_id'] == 2]['xyz_class'].values[0], 'Y')  # 0.5 <= CV < 1.0
        self.assertEqual(result[result['product_id'] == 3]['xyz_class'].values[0], 'Z')  # CV >= 1.0

    def test_recommendations_exist(self):
        """Test that all 9 ABC-XYZ combinations have recommendations"""
        classes = ['AX', 'AY', 'AZ', 'BX', 'BY', 'BZ', 'CX', 'CY', 'CZ']

        for cls in classes:
            self.assertIn(cls, self.classifier.RECOMMENDATIONS)
            rec = self.classifier.RECOMMENDATIONS[cls]
            self.assertIn('strategy', rec)
            self.assertIn('priority', rec)
            self.assertIn('actions', rec)


class TestProductClusterer(unittest.TestCase):
    """Tests for ProductClusterer model"""

    def setUp(self):
        self.clusterer = ProductClusterer()

    def test_find_optimal_clusters(self):
        """Test optimal cluster detection"""
        # Generate synthetic data with 3 clear clusters
        np.random.seed(42)
        cluster1 = np.random.randn(20, 6) + [0, 0, 0, 0, 0, 0]
        cluster2 = np.random.randn(20, 6) + [5, 5, 5, 5, 5, 5]
        cluster3 = np.random.randn(20, 6) + [10, 10, 10, 10, 10, 10]
        features = np.vstack([cluster1, cluster2, cluster3])

        n_clusters = self.clusterer._find_optimal_clusters(features, max_clusters=8)

        # Should detect around 3 clusters
        self.assertGreaterEqual(n_clusters, 2)
        self.assertLessEqual(n_clusters, 5)

    def test_feature_columns(self):
        """Test that required feature columns are defined"""
        expected = [
            'revenue_score', 'variability_score', 'trend_score',
            'seasonality_score', 'frequency_score', 'margin_score'
        ]

        self.assertEqual(self.clusterer.FEATURE_COLUMNS, expected)


class TestSupplierScorer(unittest.TestCase):
    """Tests for SupplierScorer model"""

    def setUp(self):
        self.scorer = SupplierScorer()

    def test_scoring_weights(self):
        """Test that scoring weights sum to 1.0"""
        total_weight = sum(self.scorer.WEIGHTS.values())
        self.assertAlmostEqual(total_weight, 1.0, places=4)

    def test_delivery_score_calculation(self):
        """Test delivery performance score calculation"""
        data = {
            'total_restocks': 10,
            'received_count': 9,
            'cancelled_count': 1,
            'lead_times': [3, 4, 3, 5, 3, 4, 3, 4, 3]
        }

        score, metrics = self.scorer._calc_delivery(data)

        self.assertEqual(score, 90.0)  # 9/10 * 100
        self.assertEqual(metrics['on_time_count'], 9)
        self.assertEqual(metrics['total_deliveries'], 10)

    def test_quality_score_calculation(self):
        """Test quality score calculation"""
        data = {
            'total_restocks': 10,
            'received_count': 8,
            'cancelled_count': 2,
            'lead_times': []
        }

        score, metrics = self.scorer._calc_quality(data)

        self.assertEqual(score, 80.0)  # 100 - (2/10 * 100)
        self.assertEqual(metrics['defect_count'], 2)

    def test_lead_time_score_calculation(self):
        """Test lead time consistency score"""
        # Fast and consistent lead times
        data = {'lead_times': [3, 3, 3, 3, 3]}
        score, metrics = self.scorer._calc_lead_time(data)
        self.assertGreater(score, 80)  # Should be high

        # Slow and inconsistent lead times
        data = {'lead_times': [5, 15, 8, 20, 3]}
        score, metrics = self.scorer._calc_lead_time(data)
        self.assertLess(score, 80)  # Should be lower

    def test_rating_thresholds(self):
        """Test rating assignment based on score"""
        test_cases = [
            (95, 'EXCELLENT'),
            (80, 'GOOD'),
            (65, 'ACCEPTABLE'),
            (50, 'POOR'),
            (30, 'UNACCEPTABLE'),
        ]

        for overall_score, expected_rating in test_cases:
            rating, _ = self.scorer._generate_recommendation(
                overall_score, 80, 80, 80, 80
            )
            self.assertEqual(rating, expected_rating, f"Score {overall_score} should be {expected_rating}")


# =============================================================================
# Tests for Friend's Models (PriceAnomalyDetector, SalesAnomalyDetector, PriceSuggester)
# =============================================================================

class TestPriceAnomalyDetector(unittest.TestCase):
    """Tests for PriceAnomalyDetector model"""

    def setUp(self):
        self.detector = PriceAnomalyDetector(model_dir="/tmp/test_models", cache_days=0)

    def test_minimum_data_requirement(self):
        """Test that detector requires minimum data"""
        result = self.detector.predict([])
        self.assertEqual(result, [])

        result = self.detector.predict([{"product_id": 1, "price": 100}])
        self.assertEqual(result, [])

    def test_anomaly_detection_basic(self):
        """Test basic anomaly detection"""
        # Create price history with one obvious anomaly
        price_history = [
            {"product_id": 1, "price": 100, "buying_price": 50},
            {"product_id": 1, "price": 102, "buying_price": 50},
            {"product_id": 1, "price": 98, "buying_price": 50},
            {"product_id": 1, "price": 101, "buying_price": 50},
            {"product_id": 1, "price": 500, "buying_price": 50},  # Anomaly!
            {"product_id": 2, "price": 200, "buying_price": 100},
            {"product_id": 2, "price": 205, "buying_price": 100},
            {"product_id": 2, "price": 198, "buying_price": 100},
        ]

        results = self.detector.predict(price_history)

        self.assertEqual(len(results), 8)
        # Each result should have required fields
        for result in results:
            self.assertIn("product_id", result)
            self.assertIn("current_price", result)
            self.assertIn("expected_price", result)
            self.assertIn("anomaly_score", result)
            self.assertIn("is_anomaly", result)

    def test_per_product_expected_price(self):
        """Test that expected price is calculated per product"""
        price_history = [
            {"product_id": 1, "price": 100, "buying_price": 50},
            {"product_id": 1, "price": 100, "buying_price": 50},
            {"product_id": 2, "price": 500, "buying_price": 250},
            {"product_id": 2, "price": 500, "buying_price": 250},
        ]

        results = self.detector.predict(price_history)

        # Product 1's expected price should be around 100
        product1_results = [r for r in results if r["product_id"] == 1]
        self.assertAlmostEqual(product1_results[0]["expected_price"], 100, delta=5)

        # Product 2's expected price should be around 500
        product2_results = [r for r in results if r["product_id"] == 2]
        self.assertAlmostEqual(product2_results[0]["expected_price"], 500, delta=5)

    def test_clear_cache(self):
        """Test that cache can be cleared"""
        # Should not raise an error even if no cache exists
        self.detector.clear_cache()


class TestSalesAnomalyDetector(unittest.TestCase):
    """Tests for SalesAnomalyDetector model"""

    def setUp(self):
        self.detector = SalesAnomalyDetector(model_dir="/tmp/test_models", cache_days=0)

    def test_minimum_data_requirement(self):
        """Test that detector requires minimum data"""
        result = self.detector.predict([])
        self.assertEqual(result, [])

        result = self.detector.predict([{"product_id": 1, "sales_volume": 100}])
        self.assertEqual(result, [])

    def test_anomaly_detection_basic(self):
        """Test basic anomaly detection"""
        sales_data = [
            {"product_id": 1, "sales_volume": 100, "order_count": 10},
            {"product_id": 2, "sales_volume": 150, "order_count": 15},
            {"product_id": 3, "sales_volume": 120, "order_count": 12},
            {"product_id": 4, "sales_volume": 110, "order_count": 11},
            {"product_id": 5, "sales_volume": 1000, "order_count": 5},  # Potential anomaly
        ]

        results = self.detector.predict(sales_data)

        self.assertEqual(len(results), 5)
        # Each result should have required fields
        for result in results:
            self.assertIn("product_id", result)
            self.assertIn("sales_volume", result)
            self.assertIn("expected_sales", result)
            self.assertIn("anomaly_score", result)
            self.assertIn("is_anomaly", result)

    def test_clear_cache(self):
        """Test that cache can be cleared"""
        self.detector.clear_cache()


class TestPriceSuggester(unittest.TestCase):
    """Tests for PriceSuggester model"""

    def setUp(self):
        self.suggester = PriceSuggester(model_dir="/tmp/test_models", cache_days=0)

    def test_no_price_history(self):
        """Test handling of empty price history"""
        products = [{"product_id": 1, "current_price": 100}]
        result = self.suggester.predict(products, [])
        self.assertEqual(result, [])

    def test_insufficient_training_data(self):
        """Test handling of insufficient training data"""
        products = [{"product_id": 1, "current_price": 100}]
        price_history = [
            {"product_id": 1, "price": 100},
            {"product_id": 1, "price": 105},
        ]
        result = self.suggester.predict(products, price_history)
        self.assertEqual(result, [])

    def test_price_suggestion_generation(self):
        """Test that price suggestions are generated with sufficient data"""
        products = [
            {"product_id": 1, "current_price": 100},
            {"product_id": 2, "current_price": 200},
        ]
        # Create enough price history for training
        price_history = []
        for i in range(20):
            price_history.append({"product_id": 1, "price": 100 + (i % 5)})
            price_history.append({"product_id": 2, "price": 200 + (i % 10)})

        results = self.suggester.predict(products, price_history)

        # Should generate suggestions for both products
        self.assertGreaterEqual(len(results), 1)

        for result in results:
            self.assertIn("product_id", result)
            self.assertIn("suggested_price", result)
            self.assertIn("current_price", result)
            self.assertIn("confidence", result)
            self.assertIn("reason", result)
            # Suggested price should be reasonable (within 50% of current)
            self.assertGreater(result["suggested_price"], result["current_price"] * 0.5)
            self.assertLess(result["suggested_price"], result["current_price"] * 1.5)

    def test_confidence_calculation(self):
        """Test that confidence is calculated correctly"""
        products = [{"product_id": 1, "current_price": 100}]

        # Create price history with many data points
        price_history = [{"product_id": 1, "price": 100 + (i % 5)} for i in range(50)]

        results = self.suggester.predict(products, price_history)

        if results:
            # Confidence should be between 0 and 1
            self.assertGreaterEqual(results[0]["confidence"], 0)
            self.assertLessEqual(results[0]["confidence"], 1)

    def test_clear_cache(self):
        """Test that cache can be cleared"""
        self.suggester.clear_cache()


# =============================================================================
# Integration Tests
# =============================================================================

class TestModelIntegration(unittest.TestCase):
    """Integration tests for model interactions"""

    def test_classification_recommendation_keys(self):
        """Test that classification recommendations have required keys"""
        classifier = ABCXYZClassifier()

        for abc_xyz, rec in classifier.RECOMMENDATIONS.items():
            self.assertIn('strategy', rec, f"{abc_xyz} missing strategy")
            self.assertIn('priority', rec, f"{abc_xyz} missing priority")
            self.assertIn('actions', rec, f"{abc_xyz} missing actions")
            self.assertIsInstance(rec['actions'], list, f"{abc_xyz} actions should be list")

    def test_all_models_instantiate(self):
        """Test that all models can be instantiated"""
        models = [
            DemandForecaster(model_dir="/tmp/test"),
            ABCXYZClassifier(),
            ProductClusterer(),
            SupplierScorer(),
            PriceAnomalyDetector(model_dir="/tmp/test"),
            SalesAnomalyDetector(model_dir="/tmp/test"),
            PriceSuggester(model_dir="/tmp/test"),
        ]

        self.assertEqual(len(models), 7)


if __name__ == '__main__':
    unittest.main()
