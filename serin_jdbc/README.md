# Serin JDBC Driver

Usage:
```java
import java.sql.*;
import io.serindb.jdbc.SerinDriver;

public class Example {
    public static void main(String[] args) throws Exception {
        Class.forName("io.serindb.jdbc.SerinDriver");
        Connection conn = DriverManager.getConnection("jdbc:serin://localhost:5432/", "alice", "password");
        Statement st = conn.createStatement();
        ResultSet rs = st.executeQuery("SELECT 1");
        rs.next();
        System.out.println(rs.getInt(1));
        conn.close();
    }
}
``` 