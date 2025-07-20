// Package driver provides a database/sql compatible driver for SerinDB.
package driver

import (
    "context"
    "database/sql"
    "database/sql/driver"
    "github.com/jackc/pgx/v5"
)

func init() {
    sql.Register("serin", &serinDriver{})
}

type serinDriver struct{}

func (d *serinDriver) Open(name string) (driver.Conn, error) {
    cfg, err := pgx.ParseConfig(name)
    if err != nil {
        return nil, err
    }
    conn, err := pgx.ConnectConfig(context.Background(), cfg)
    if err != nil {
        return nil, err
    }
    return &serinConn{conn: conn}, nil
}

type serinConn struct {
    conn *pgx.Conn
}

func (c *serinConn) Prepare(query string) (driver.Stmt, error) {
    return &serinStmt{conn: c.conn, query: query}, nil
}

func (c *serinConn) Close() error { return c.conn.Close(context.Background()) }

func (c *serinConn) Begin() (driver.Tx, error) { return nil, driver.ErrSkip }

// serinStmt implements driver.Stmt

type serinStmt struct {
    conn  *pgx.Conn
    query string
}

func (s *serinStmt) Close() error { return nil }

func (s *serinStmt) NumInput() int { return -1 }

func (s *serinStmt) Exec(args []driver.Value) (driver.Result, error) {
    ct, err := s.conn.Exec(context.Background(), s.query).RowsAffected(), error(nil)
    return driver.RowsAffected(ct), err
}

func (s *serinStmt) Query(args []driver.Value) (driver.Rows, error) {
    rows, err := s.conn.Query(context.Background(), s.query)
    if err != nil {
        return nil, err
    }
    return &serinRows{pgRows: rows}, nil
}

// serinRows wraps pgx.Rows to implement driver.Rows

type serinRows struct {
    pgRows pgx.Rows
}

func (r *serinRows) Columns() []string {
    flds := r.pgRows.FieldDescriptions()
    cols := make([]string, len(flds))
    for i, f := range flds { cols[i] = string(f.Name) }
    return cols
}

func (r *serinRows) Close() error { r.pgRows.Close(); return nil }

func (r *serinRows) Next(dest []driver.Value) error {
    if !r.pgRows.Next() { return driver.ErrBadConn }
    values, err := r.pgRows.Values()
    if err != nil { return err }
    copy(dest, values)
    return nil
} 