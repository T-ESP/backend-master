"""Tests for AI service Flask API endpoints."""
import unittest
from unittest.mock import patch, MagicMock
from main import app


class TestHealthEndpoint(unittest.TestCase):
    def setUp(self):
        self.client = app.test_client()
        app.config['TESTING'] = True

    @patch('main.check_database_health')
    def test_health_returns_200_when_healthy(self, mock_health):
        mock_health.return_value = {"status": "healthy", "database": "connected", "version": "PostgreSQL 16"}
        response = self.client.get('/ai/health')
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertIn('status', data)
        self.assertEqual(data['status'], 'healthy')

    @patch('main.check_database_health')
    def test_health_returns_degraded_when_db_down(self, mock_health):
        mock_health.return_value = {"status": "unhealthy", "database": "disconnected", "error": "connection refused"}
        response = self.client.get('/ai/health')
        data = response.get_json()
        self.assertIn('status', data)
        self.assertEqual(data['status'], 'degraded')


class TestStatusEndpoint(unittest.TestCase):
    def setUp(self):
        self.client = app.test_client()
        app.config['TESTING'] = True

    @patch('main.get_metrics')
    def test_status_returns_200(self, mock_metrics):
        mock_metrics.return_value = None
        response = self.client.get('/ai/status')
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertIn('last_run', data)
        self.assertIn('is_running', data)

    @patch('main.get_metrics')
    def test_status_returns_last_run_metrics(self, mock_metrics):
        mock_metrics.return_value = {
            "run_started": "2026-03-27T02:00:00",
            "duration_seconds": 42.5,
            "successful": 7,
            "failed": 0
        }
        response = self.client.get('/ai/status')
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertIsNotNone(data['last_run'])


class TestRunEndpoint(unittest.TestCase):
    def setUp(self):
        self.client = app.test_client()
        app.config['TESTING'] = True

    @patch('main.run_all_jobs')
    def test_run_returns_202(self, mock_run):
        response = self.client.post('/ai/run')
        self.assertEqual(response.status_code, 202)
        data = response.get_json()
        self.assertIn('message', data)
        self.assertEqual(data['status'], 'started')


if __name__ == '__main__':
    unittest.main()
