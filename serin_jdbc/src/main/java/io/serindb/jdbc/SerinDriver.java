package io.serindb.jdbc;

import java.sql.*;
import java.util.Properties;
import java.util.logging.Logger;

/**
 * Serin JDBC Driver.
 * A thin wrapper around PostgreSQL JDBC driver that sets default
 * protocol options for SerinDB.
 */
public class SerinDriver implements Driver {
    private static final String PREFIX = "jdbc:serin:";
    private static final String PG_PREFIX = "jdbc:postgresql:";
    private static final Driver PG_DRIVER;

    static {
        try {
            PG_DRIVER = (Driver) Class.forName("org.postgresql.Driver").getDeclaredConstructor().newInstance();
            DriverManager.registerDriver(new SerinDriver());
        } catch (Exception e) {
            throw new RuntimeException("Failed to load PostgreSQL driver", e);
        }
    }

    @Override
    public Connection connect(String url, Properties info) throws SQLException {
        if (!acceptsURL(url)) return null;
        String pgUrl = url.replace(PREFIX, PG_PREFIX);
        // default port 5432 if not specified
        return PG_DRIVER.connect(pgUrl, info);
    }

    @Override
    public boolean acceptsURL(String url) {
        return url != null && url.startsWith(PREFIX);
    }

    @Override
    public DriverPropertyInfo[] getPropertyInfo(String url, Properties info) throws SQLException {
        return PG_DRIVER.getPropertyInfo(url, info);
    }

    @Override
    public int getMajorVersion() { return 1; }

    @Override
    public int getMinorVersion() { return 0; }

    @Override
    public boolean jdbcCompliant() { return true; }

    @Override
    public Logger getParentLogger() {
        return Logger.getLogger("io.serindb.jdbc");
    }
} 