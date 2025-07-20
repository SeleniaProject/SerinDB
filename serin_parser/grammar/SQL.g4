grammar SQL;

select_stmt: SELECT select_elem (COMMA select_elem)* SEMI?;

select_elem: STAR | NUMBER;

SELECT: [sS][eE][lL][eE][cC][tT];
FROM: [fF][rR][oO][mM];
WHERE: [wW][hH][eE][rR][eE];
STAR: '*';
COMMA: ',';
SEMI: ';';
NUMBER: [0-9]+;
WS: [ \t\r\n]+ -> skip; 