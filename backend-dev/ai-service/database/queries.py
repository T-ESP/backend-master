def get_products_with_prices(conn):
    query = """
        SELECT DISTINCT ON (p.id_pro)
            p.id_pro AS product_id,
            p.name_pro AS product_name,
            prp.price_prp AS current_price,
            p.buying_price_pro AS buying_price
        FROM products_pro p
        JOIN productprices_prp prp ON p.id_pro = prp.product_ref_prp
        WHERE prp.price_prp IS NOT NULL
        ORDER BY p.id_pro, prp.created_at DESC
    """
    with conn.cursor() as cur:
        cur.execute(query)
        columns = [desc[0] for desc in cur.description]
        return [dict(zip(columns, row)) for row in cur.fetchall()]


def get_price_history(conn):
    query = """
        SELECT
            prp.product_ref_prp AS product_id,
            prp.price_prp AS price,
            p.buying_price_pro AS buying_price,
            prp.created_at AS recorded_at
        FROM productprices_prp prp
        JOIN products_pro p ON prp.product_ref_prp = p.id_pro
        ORDER BY prp.product_ref_prp, prp.created_at
    """
    with conn.cursor() as cur:
        cur.execute(query)
        columns = [desc[0] for desc in cur.description]
        return [dict(zip(columns, row)) for row in cur.fetchall()]


def get_sales_data(conn):
    query = """
        SELECT
            lor.product_id_lor AS product_id,
            SUM(lor.quantity_lor) AS sales_volume,
            COUNT(DISTINCT o.id_ord) AS order_count
        FROM line_order_lor lor
        JOIN order_ord o ON lor.order_id_lor = o.id_ord
        GROUP BY lor.product_id_lor
    """
    with conn.cursor() as cur:
        cur.execute(query)
        columns = [desc[0] for desc in cur.description]
        return [dict(zip(columns, row)) for row in cur.fetchall()]
