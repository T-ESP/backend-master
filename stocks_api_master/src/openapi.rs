use utoipa::OpenApi;

use crate::features::products::dto::{
    ProductResponse, CreateProductRequest, UpdateProductRequest, ProductLightResponse,
    StockUpdateRequest,
};
use crate::features::products::kpis_dto::{
    PricingMarginKpis, StockAvailabilityKpis, SalesRotationKpis,
    ProfitabilityKpis, RestockKpis, PredictionsAlertsKpis,
    ScoringClassificationKpis, ComparativeKpis, PriceEvolution,
    PricePoint, MarginPoint, AllProductKpis
};

use crate::features::orders::dto::{
    OrderResponse, CreateOrderRequest, CreateLineItemRequest, LineItemResponse,
    OrderWithItemsResponse, UpdateOrderRequest, OrderStatsResponse
};
use crate::features::users::dto::{
    UserResponse, CreateUserRequest, UpdateUserRequest
};
use crate::features::auth::dto::{LoginRequest, LoginResponse, AdminRegisterRequest};
use crate::features::suppliers::dto::{
    SupplierResponse, CreateSupplierRequest, UpdateSupplierRequest
};
use crate::features::stocks::dto::{
    StockResponse, StockAlert, StockSummary
};
use crate::features::sales::dto::{
    TotalRevenueResponse, EvolutionResponse, ComparisonResponse,
    AverageBasketResponse, AverageBasketByClientTypeResponse
};
use crate::features::restocks::dto::{
    RestockResponse, RestockWithSupplierResponse, RestockStatsResponse,
    CreateRestockRequest, UpdateRestockRequest, LineRestockResponse, LineRestockWithSupplierResponse,
    CreateLineRestockRequest, RestockStatus
};
use crate::features::global_kpis::dto::{
    GlobalPerformanceKpis, CategoryAnalysisKpis, CategoryStats,
    SupplierAnalysisKpis, SupplierStats, CatalogHealthKpis,
    AbcDistributionKpis, AbcProductInfo, TrendsKpis,
    OperationalEfficiencyKpis, PriceAnalysisKpis, MarginDistribution,
    TopFlopKpis, RankingProductInfo, ForecastKpis,
    OptimizationOpportunity, TimeSeriesKpis, TimeSeriesPoint
};
use crate::features::alerts::dto::{
    AlertResponse, AlertSummary, StatusCounts, SeverityCounts, ModelTypeCounts,
    UpdateStatusRequest, BulkUpdateStatusRequest
};
use crate::features::ai_predictions::dto::{
    ForecastResponse, ClassificationResponse, ClusterResponse,
    SupplierScoreResponse, PriceSuggestionResponse, PriceAnomalyResponse,
    SalesAnomalyResponse, UrgentRestockResponse
};
use crate::features::ai_insights::dto::{AiInsightsData, PeriodInfo};
use crate::features::tenants::dto::{TenantResponse, CreateTenantRequest, UpdateTenantRequest};
use crate::common::responses::{ErrorResponse, ErrorInfo};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Tenants (plateforme)
        crate::features::tenants::handler::create_tenant,
        crate::features::tenants::handler::get_all_tenants,
        crate::features::tenants::handler::get_tenant_by_id,
        crate::features::tenants::handler::update_tenant,
        crate::features::tenants::handler::delete_tenant,
        crate::features::tenants::handler::seed_tenant,
        crate::features::tenants::handler::verify_tenant,
        // Products
        crate::features::products::handlers::get_products,
        crate::features::products::handlers::search_products_light,
        crate::features::products::handlers::get_product_by_id,
        crate::features::products::handlers::get_product_stock,
        crate::features::products::handlers::update_stock,
        crate::features::products::handlers::create_product,
        crate::features::products::handlers::update_product,
        crate::features::products::handlers::delete_product,
        // Product KPIs
        crate::features::products::kpis_handlers::get_all_product_kpis,
        crate::features::products::kpis_handlers::get_pricing_margin_kpis,
        crate::features::products::kpis_handlers::get_stock_availability_kpis,
        crate::features::products::kpis_handlers::get_sales_rotation_kpis,
        crate::features::products::kpis_handlers::get_profitability_kpis,
        crate::features::products::kpis_handlers::get_restock_kpis,
        crate::features::products::kpis_handlers::get_predictions_alerts_kpis,
        crate::features::products::kpis_handlers::get_scoring_classification_kpis,
        crate::features::products::kpis_handlers::get_comparative_kpis,
        crate::features::products::kpis_handlers::get_price_evolution,
        // Orders
        crate::features::orders::handlers::get_orders,
        crate::features::orders::handlers::create_order,
        crate::features::orders::handlers::get_order_by_id,
        crate::features::orders::handlers::update_order,
        crate::features::orders::handlers::delete_order,
        crate::features::orders::handlers::get_order_items,
        crate::features::orders::handlers::get_orders_by_user,
        crate::features::orders::handlers::get_order_stats,
        // Users
        crate::features::users::handlers::get_users,
        crate::features::users::handlers::create_user,
        crate::features::users::handlers::update_user,
        crate::features::users::handlers::delete_user,
        // Suppliers
        crate::features::suppliers::handlers::get_all_suppliers,
        crate::features::suppliers::handlers::create_supplier,
        crate::features::suppliers::handlers::get_supplier_by_id,
        crate::features::suppliers::handlers::get_supplier_by_email,
        crate::features::suppliers::handlers::update_supplier,
        crate::features::suppliers::handlers::delete_supplier,
        // Stocks
        crate::features::stocks::handlers::get_out_of_stock,
        crate::features::stocks::handlers::get_low_stock,
        crate::features::stocks::handlers::get_soon_out_of_stock,
        crate::features::stocks::handlers::get_overstock,
        crate::features::stocks::handlers::get_stock_alerts,
        crate::features::stocks::handlers::get_stock_summary,
        // Sales
        crate::features::sales::handlers::get_total_revenue,
        crate::features::sales::handlers::get_evolution,
        crate::features::sales::handlers::get_comparison,
        crate::features::sales::handlers::get_average_basket,
        crate::features::sales::handlers::get_average_basket_by_client_type,
        // Auth
        crate::features::auth::handlers::login,
        crate::features::auth::handlers::register,
        // Restocks
        crate::features::restocks::handlers::get_restocks,
        crate::features::restocks::handlers::get_restock_by_id,
        crate::features::restocks::handlers::get_restocks_with_supplier,
        crate::features::restocks::handlers::create_restock,
        crate::features::restocks::handlers::update_restock,
        crate::features::restocks::handlers::get_restock_stats,
        crate::features::restocks::handlers::get_restock_stats_by_product,
        crate::features::restocks::handlers::get_restocks_by_product,
        // Global KPIs
        crate::features::global_kpis::handlers::get_global_performance_kpis,
        crate::features::global_kpis::handlers::get_category_analysis_kpis,
        crate::features::global_kpis::handlers::get_supplier_analysis_kpis,
        crate::features::global_kpis::handlers::get_catalog_health_kpis,
        crate::features::global_kpis::handlers::get_abc_distribution_kpis,
        crate::features::global_kpis::handlers::get_trends_kpis,
        crate::features::global_kpis::handlers::get_operational_efficiency_kpis,
        crate::features::global_kpis::handlers::get_price_analysis_kpis,
        crate::features::global_kpis::handlers::get_top_flop_kpis,
        crate::features::global_kpis::handlers::get_forecast_kpis,
        crate::features::global_kpis::handlers::get_time_series_kpis,
        // Alerts
        crate::features::alerts::handlers::get_alerts,
        crate::features::alerts::handlers::get_alert_summary,
        crate::features::alerts::handlers::get_alert_by_id,
        crate::features::alerts::handlers::update_alert_status,
        crate::features::alerts::handlers::bulk_update_status,
        crate::features::alerts::handlers::cleanup_alerts,
        // AI Predictions
        crate::features::ai_predictions::handlers::get_forecasts,
        crate::features::ai_predictions::handlers::get_forecast_by_product,
        crate::features::ai_predictions::handlers::get_classifications,
        crate::features::ai_predictions::handlers::get_classification_by_product,
        crate::features::ai_predictions::handlers::get_clusters,
        crate::features::ai_predictions::handlers::get_cluster_by_product,
        crate::features::ai_predictions::handlers::get_supplier_scores,
        crate::features::ai_predictions::handlers::get_supplier_score,
        crate::features::ai_predictions::handlers::get_price_suggestions,
        crate::features::ai_predictions::handlers::get_price_anomalies,
        crate::features::ai_predictions::handlers::get_sales_anomalies,
        crate::features::ai_predictions::handlers::get_urgent_restocks,
        // AI Insights
        crate::features::ai_insights::handlers::get_ai_insights,
    ),
    components(
        schemas(
            // Common
            ErrorResponse,
            ErrorInfo,
            // Auth
            LoginRequest,
            LoginResponse,
            AdminRegisterRequest,
            // Tenants
            TenantResponse,
            CreateTenantRequest,
            UpdateTenantRequest,
            // Products
            ProductResponse,
            ProductLightResponse,
            CreateProductRequest,
            UpdateProductRequest,
            StockUpdateRequest,
            // Product KPIs
            AllProductKpis,
            PricingMarginKpis,
            StockAvailabilityKpis,
            SalesRotationKpis,
            ProfitabilityKpis,
            RestockKpis,
            PredictionsAlertsKpis,
            ScoringClassificationKpis,
            ComparativeKpis,
            PriceEvolution,
            PricePoint,
            MarginPoint,
            // Orders
            OrderResponse,
            CreateOrderRequest,
            CreateLineItemRequest,
            LineItemResponse,
            OrderWithItemsResponse,
            UpdateOrderRequest,
            OrderStatsResponse,
            // Users
            UserResponse,
            CreateUserRequest,
            UpdateUserRequest,
            // Suppliers
            SupplierResponse,
            CreateSupplierRequest,
            UpdateSupplierRequest,
            // Stocks
            StockResponse,
            StockAlert,
            StockSummary,
            // Sales
            TotalRevenueResponse,
            EvolutionResponse,
            ComparisonResponse,
            AverageBasketResponse,
            AverageBasketByClientTypeResponse,
            // Restocks
            RestockResponse,
            RestockWithSupplierResponse,
            RestockStatsResponse,
            CreateRestockRequest,
            UpdateRestockRequest,
            LineRestockResponse,
            LineRestockWithSupplierResponse,
            CreateLineRestockRequest,
            RestockStatus,
            // Global KPIs
            GlobalPerformanceKpis,
            CategoryAnalysisKpis,
            CategoryStats,
            SupplierAnalysisKpis,
            SupplierStats,
            CatalogHealthKpis,
            AbcDistributionKpis,
            AbcProductInfo,
            TrendsKpis,
            OperationalEfficiencyKpis,
            PriceAnalysisKpis,
            MarginDistribution,
            TopFlopKpis,
            RankingProductInfo,
            ForecastKpis,
            OptimizationOpportunity,
            TimeSeriesKpis,
            TimeSeriesPoint,
            // Alerts
            AlertResponse,
            AlertSummary,
            StatusCounts,
            SeverityCounts,
            ModelTypeCounts,
            UpdateStatusRequest,
            BulkUpdateStatusRequest,
            // AI Predictions
            ForecastResponse,
            ClassificationResponse,
            ClusterResponse,
            SupplierScoreResponse,
            PriceSuggestionResponse,
            PriceAnomalyResponse,
            SalesAnomalyResponse,
            UrgentRestockResponse,
            // AI Insights
            AiInsightsData,
            PeriodInfo
        )
    ),
    tags(
        (name = "tenants", description = "Gestion des commerces (admin plateforme) et vérification de slug"),
        (name = "products", description = "Product management endpoints"),
        (name = "product-kpis", description = "Product KPIs and analytics endpoints"),
        (name = "orders", description = "Order management endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "suppliers", description = "Supplier management endpoints"),
        (name = "stocks", description = "Stock monitoring endpoints"),
        (name = "sales", description = "Sales analytics endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "restocks", description = "Restock management endpoints"),
        (name = "global-kpis", description = "Global KPIs and analytics endpoints"),
        (name = "alerts", description = "Alert and notification management endpoints"),
        (name = "ai_predictions", description = "AI model predictions and results endpoints"),
        (name = "ai-insights", description = "Aggregated AI insights endpoint"),
    ),
    info(
        title = "T-ESP Stock Management API",
        version = "1.0.0",
        description = "API multi-tenant pour la gestion des stocks (routes métier sous /api/{commerce_id}/...).",
        contact(
            name = "API Support",
            email = "support@example.com"
        ),
        license(
            name = "MIT",
        )
    )
)]
pub struct ApiDoc;
