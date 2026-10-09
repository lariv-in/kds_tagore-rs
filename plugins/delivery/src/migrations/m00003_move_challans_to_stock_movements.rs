//! Copy delivery challans into inventory stock movements, then drop the challan tables.
//!
//! Runs while `customers` still exists. Each challan becomes an outward movement
//! (`movement_type = 'out'`): a delivery challan is stock leaving inventory.
//! The bill-to party is an existing company or contact when one already matches
//! that customer (GSTIN, then name; for a person, name plus phone and email).
//! Otherwise the party is created from the customer. `legacy_customer_id` is
//! stamped on the party so the later invoice migration reuses it.
//! Lines become movement lines, and a stock is created per product, quantity
//! kind, and the customer's company. Lengths are stored in the line's display
//! unit (the challan column is millimetres). Weights stay in kilograms. Whole
//! numbers stay whole numbers.
//! `eway_bill` is copied onto the stock movement. Company presentation fields
//! are copied onto inventory preferences when those fields are still empty.
//! The Typst challan template is not copied.

use lariv_core::db::migration_sql::exec_sql;
use sea_orm::{ConnectionTrait, DatabaseBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_POSTGRES: &str = r#"
DO $migrate$
DECLARE
    cust record;
    party_id bigint;
    missing_party bigint;
BEGIN
    IF to_regclass('public.delivery_challans') IS NULL THEN
        RETURN;
    END IF;

    IF EXISTS (SELECT 1 FROM delivery_challans) THEN
        IF to_regclass('public.customers') IS NULL THEN
            RAISE EXCEPTION 'customers is already gone; cannot match delivery challan parties';
        END IF;

        ALTER TABLE crm_companies ADD COLUMN IF NOT EXISTS legacy_customer_id bigint;
        ALTER TABLE crm_contacts ADD COLUMN IF NOT EXISTS legacy_customer_id bigint;

        CREATE TEMP TABLE challan_party (
            customer_id bigint PRIMARY KEY,
            bill_to_individual boolean NOT NULL,
            contact_id bigint,
            company_id bigint,
            CONSTRAINT challan_party_one CHECK (
                (bill_to_individual AND contact_id IS NOT NULL AND company_id IS NULL)
                OR (NOT bill_to_individual AND company_id IS NOT NULL AND contact_id IS NULL)
            )
        ) ON COMMIT DROP;

        FOR cust IN
            SELECT *
            FROM customers
            WHERE id IN (SELECT customer_id FROM delivery_challans)
        LOOP
            party_id := NULL;

            IF cust.customer_type = 'individual' THEN
                SELECT c.id INTO party_id
                FROM crm_contacts AS c
                WHERE c.legacy_customer_id = cust.id
                ORDER BY c.id
                LIMIT 1;

                IF party_id IS NULL THEN
                    SELECT c.id INTO party_id
                    FROM crm_contacts AS c
                    WHERE lower(btrim(c.name)) = lower(btrim(cust.name))
                      AND (
                          NULLIF(btrim(cust.phone), '') IS NULL
                          OR btrim(COALESCE(c.phone, '')) = btrim(cust.phone)
                      )
                      AND (
                          NULLIF(btrim(cust.email), '') IS NULL
                          OR lower(btrim(COALESCE(c.email, ''))) = lower(btrim(cust.email))
                      )
                    ORDER BY (c.company_id IS NULL) DESC, c.id
                    LIMIT 1;
                END IF;

                IF party_id IS NULL THEN
                    INSERT INTO crm_contacts (
                        name, email, phone, is_primary, company_id,
                        created_at, updated_at, legacy_customer_id
                    ) VALUES (
                        cust.name,
                        NULLIF(btrim(cust.email), ''),
                        NULLIF(btrim(cust.phone), ''),
                        FALSE,
                        NULL,
                        COALESCE(cust.created_at, now()),
                        COALESCE(cust.updated_at, now()),
                        cust.id
                    )
                    RETURNING id INTO party_id;
                ELSE
                    UPDATE crm_contacts
                    SET legacy_customer_id = cust.id
                    WHERE id = party_id
                      AND legacy_customer_id IS NULL;
                END IF;

                INSERT INTO challan_party (customer_id, bill_to_individual, contact_id, company_id)
                VALUES (cust.id, TRUE, party_id, NULL);
            ELSE
                SELECT c.id INTO party_id
                FROM crm_companies AS c
                WHERE c.legacy_customer_id = cust.id
                ORDER BY c.id
                LIMIT 1;

                IF party_id IS NULL AND NULLIF(btrim(cust.gstin), '') IS NOT NULL THEN
                    SELECT c.id INTO party_id
                    FROM crm_companies AS c
                    WHERE lower(btrim(c.gstin)) = lower(btrim(cust.gstin))
                    ORDER BY (c.legacy_customer_id IS NULL) DESC, c.id
                    LIMIT 1;
                END IF;

                IF party_id IS NULL THEN
                    SELECT c.id INTO party_id
                    FROM crm_companies AS c
                    WHERE lower(btrim(c.name)) = lower(btrim(cust.name))
                      AND (
                          NULLIF(btrim(cust.gstin), '') IS NULL
                          OR NULLIF(btrim(c.gstin), '') IS NULL
                          OR lower(btrim(c.gstin)) = lower(btrim(cust.gstin))
                      )
                    ORDER BY (c.legacy_customer_id IS NULL) DESC, c.id
                    LIMIT 1;
                END IF;

                IF party_id IS NULL THEN
                    INSERT INTO crm_companies (
                        name, address_line_1, address_line_2, city, pincode, state, website,
                        gstin, cin, pan, phone, email, created_at, updated_at, legacy_customer_id
                    ) VALUES (
                        cust.name,
                        cust.address_line_1,
                        cust.address_line_2,
                        cust.city,
                        cust.pincode,
                        cust.state,
                        cust.website,
                        NULLIF(btrim(cust.gstin), ''),
                        NULLIF(btrim(cust.cin), ''),
                        NULLIF(btrim(cust.pan), ''),
                        NULLIF(btrim(cust.phone), ''),
                        NULLIF(btrim(cust.email), ''),
                        COALESCE(cust.created_at, now()),
                        COALESCE(cust.updated_at, now()),
                        cust.id
                    )
                    RETURNING id INTO party_id;
                ELSE
                    UPDATE crm_companies
                    SET legacy_customer_id = cust.id
                    WHERE id = party_id
                      AND legacy_customer_id IS NULL;
                END IF;

                INSERT INTO challan_party (customer_id, bill_to_individual, contact_id, company_id)
                VALUES (cust.id, FALSE, NULL, party_id);
            END IF;
        END LOOP;

        SELECT COUNT(*) INTO missing_party
        FROM delivery_challans AS dc
        WHERE NOT EXISTS (
            SELECT 1 FROM challan_party AS p WHERE p.customer_id = dc.customer_id
        );
        IF missing_party > 0 THEN
            RAISE EXCEPTION 'delivery challan has % customer(s) missing from customers', missing_party;
        END IF;

        ALTER TABLE inventory_stock_movements ADD COLUMN IF NOT EXISTS legacy_challan_id bigint;

        INSERT INTO inventory_stock_movements (
            created_at, updated_at, datetime, movement_type, number,
            bill_to_individual, customer_individual, customer_company,
            vehicle_number, eway_bill, legacy_challan_id
        )
        SELECT
            dc.created_at,
            dc.updated_at,
            (dc.date::timestamp AT TIME ZONE 'Asia/Kolkata'),
            'out',
            CASE
                WHEN btrim(dc.challan_number) = '' THEN 'DC-' || dc.id::text
                ELSE btrim(dc.challan_number)
            END,
            p.bill_to_individual,
            p.contact_id,
            p.company_id,
            NULLIF(btrim(dc.vehicle_no), ''),
            NULLIF(btrim(dc.eway_bill), ''),
            dc.id
        FROM delivery_challans AS dc
        JOIN challan_party AS p ON p.customer_id = dc.customer_id
        WHERE NOT EXISTS (
            SELECT 1 FROM inventory_stock_movements AS existing
            WHERE existing.legacy_challan_id = dc.id
        );

        IF EXISTS (SELECT 1 FROM delivery_challan_lines) THEN
            IF EXISTS (
                SELECT 1
                FROM delivery_challan_lines AS l
                JOIN delivery_challans AS dc ON dc.id = l.delivery_challan_id
                JOIN challan_party AS party ON party.customer_id = dc.customer_id
                LEFT JOIN crm_contacts AS ct ON ct.id = party.contact_id
                WHERE COALESCE(party.company_id, ct.company_id) IS NULL
            ) THEN
                RAISE EXCEPTION 'delivery challan customer has no company to own the stock';
            END IF;

            ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS legacy_product_id bigint;
            ALTER TABLE inventory_stocks ADD COLUMN IF NOT EXISTS legacy_qty_unit text;

            INSERT INTO inventory_stocks (
                created_at, updated_at, name, company_id, qty_type, qty_unit,
                legacy_product_id, legacy_qty_unit
            )
            SELECT
                now(),
                now(),
                src.product_name,
                src.company_id,
                src.qty_kind,
                src.qty_unit,
                src.product_id,
                src.qty_unit
            FROM (
                SELECT DISTINCT
                    l.product_id,
                    p.name AS product_name,
                    l.qty_kind,
                    CASE l.qty_kind
                        WHEN 'length' THEN COALESCE(NULLIF(lower(btrim(l.qty_length_unit)), ''), 'mm')
                        WHEN 'weight' THEN 'kg'
                        ELSE ''
                    END AS qty_unit,
                    COALESCE(party.company_id, ct.company_id) AS company_id
                FROM delivery_challan_lines AS l
                JOIN products AS p ON p.id = l.product_id
                JOIN delivery_challans AS dc ON dc.id = l.delivery_challan_id
                JOIN challan_party AS party ON party.customer_id = dc.customer_id
                LEFT JOIN crm_contacts AS ct ON ct.id = party.contact_id
            ) AS src
            WHERE NOT EXISTS (
                SELECT 1 FROM inventory_stocks AS s
                WHERE s.legacy_product_id = src.product_id
                  AND s.company_id = src.company_id
                  AND s.qty_type = src.qty_kind
                  AND s.legacy_qty_unit IS NOT DISTINCT FROM src.qty_unit
            );

            INSERT INTO inventory_stock_movement_lines (
                created_at, updated_at, stock_id, stock_movement_id, qty, qty_unit, qty_type
            )
            SELECT
                l.created_at,
                l.updated_at,
                s.id,
                m.id,
                round(q.qty, 6),
                s.qty_unit,
                s.qty_type
            FROM delivery_challan_lines AS l
            JOIN inventory_stock_movements AS m ON m.legacy_challan_id = l.delivery_challan_id
            JOIN LATERAL (
                SELECT
                    CASE l.qty_kind
                        WHEN 'length' THEN COALESCE(NULLIF(lower(btrim(l.qty_length_unit)), ''), 'mm')
                        WHEN 'weight' THEN 'kg'
                        ELSE ''
                    END AS qty_unit,
                    CASE l.qty_kind
                        WHEN 'length' THEN
                            CASE COALESCE(NULLIF(lower(btrim(l.qty_length_unit)), ''), 'mm')
                                WHEN 'mm' THEN l.qty_length
                                WHEN 'cm' THEN l.qty_length / 10
                                WHEN 'm' THEN l.qty_length / 1000
                                WHEN 'km' THEN l.qty_length / 1000000
                                WHEN 'in' THEN l.qty_length / 25.4
                                WHEN 'ft' THEN l.qty_length / 304.8
                                ELSE l.qty_length
                            END
                        WHEN 'weight' THEN l.qty_weight
                        WHEN 'quantity' THEN l.qty_number::numeric
                        ELSE NULL
                    END AS qty
            ) AS q ON true
            JOIN delivery_challans AS dc ON dc.id = l.delivery_challan_id
            JOIN challan_party AS party ON party.customer_id = dc.customer_id
            LEFT JOIN crm_contacts AS ct ON ct.id = party.contact_id
            JOIN inventory_stocks AS s
                ON s.legacy_product_id = l.product_id
               AND s.company_id = COALESCE(party.company_id, ct.company_id)
               AND s.qty_type = l.qty_kind
               AND s.legacy_qty_unit IS NOT DISTINCT FROM q.qty_unit
            WHERE q.qty IS NOT NULL
              AND q.qty >= 0
            ORDER BY l.delivery_challan_id, l.sr_no, l.id;
        END IF;
    END IF;

    IF to_regclass('public.delivery_preferences') IS NOT NULL
       AND to_regclass('public.inventory_preferences') IS NOT NULL
    THEN
        UPDATE inventory_preferences AS ip
        SET
            company_name = COALESCE(ip.company_name, dp.company_name),
            company_address = COALESCE(ip.company_address, dp.company_address),
            company_phone = COALESCE(ip.company_phone, dp.company_phone),
            company_email = COALESCE(ip.company_email, dp.company_email),
            company_gstin = COALESCE(ip.company_gstin, dp.company_gstin),
            terms_and_conditions = COALESCE(ip.terms_and_conditions, dp.terms_and_conditions),
            logo_vnode_id = COALESCE(ip.logo_vnode_id, dp.logo_vnode_id),
            signature_vnode_id = COALESCE(ip.signature_vnode_id, dp.signature_vnode_id)
        FROM delivery_preferences AS dp
        WHERE ip.id = 1 AND dp.id = 1;
    END IF;

    ALTER TABLE inventory_stock_movements DROP COLUMN IF EXISTS legacy_challan_id;
    ALTER TABLE inventory_stocks DROP COLUMN IF EXISTS legacy_product_id;
    ALTER TABLE inventory_stocks DROP COLUMN IF EXISTS legacy_qty_unit;

    DROP TABLE IF EXISTS delivery_challan_lines;
    DROP TABLE IF EXISTS delivery_challans;
    DROP TABLE IF EXISTS delivery_preferences;
END
$migrate$;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::Postgres {
            return up_sqlite(manager).await;
        }
        exec_sql(manager, UP_POSTGRES).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "moving delivery challans into stock movements is not reversible".into(),
        ))
    }
}

async fn up_sqlite(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let backend = DatabaseBackend::Sqlite;
    let conn = manager.get_connection();
    if sqlite_table_exists(conn, backend, "delivery_challans").await?
        && sqlite_count(conn, backend, "SELECT COUNT(*) AS n FROM delivery_challans").await? > 0
    {
        return Err(DbErr::Custom(
            "delivery challan rows can only be moved to stock movements on PostgreSQL".into(),
        ));
    }
    for sql in [
        "DROP TABLE IF EXISTS delivery_challan_lines",
        "DROP TABLE IF EXISTS delivery_challans",
        "DROP TABLE IF EXISTS delivery_preferences",
    ] {
        exec_sql(manager, sql).await?;
    }
    Ok(())
}

async fn sqlite_table_exists(
    conn: &impl ConnectionTrait,
    backend: DatabaseBackend,
    table: &str,
) -> Result<bool, DbErr> {
    let row = conn
        .query_one_raw(Statement::from_string(
            backend,
            format!("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{table}'"),
        ))
        .await?;
    Ok(row.is_some())
}

async fn sqlite_count(
    conn: &impl ConnectionTrait,
    backend: DatabaseBackend,
    sql: &str,
) -> Result<i64, DbErr> {
    let row = conn
        .query_one_raw(Statement::from_string(backend, sql))
        .await?
        .ok_or_else(|| DbErr::Custom("sqlite count returned no row".into()))?;
    row.try_get_by_index(0)
}
