# Quickstart: evaluar la interpretación

**Feature**: `004-evaluar-interpretacion` | **Fecha**: 2026-08-19

Cómo comprobar que esta feature funciona, de principio a fin y **sin teclado ni pantalla**. Es
posible porque todo el juicio vive en `piano-core` y se alimenta de interpretaciones grabadas.

## Requisitos previos

Los mismos del proyecto: Rust 1.97.1, pnpm y `./scripts/verificar.sh` en verde antes de empezar.
Ninguna dependencia nueva; `cargo tree -p piano-core` debe seguir dando **tres líneas**.

## Las interpretaciones grabadas

`FR-022` exige fixtures de referencia con su resultado esperado, y **son la parte más valiosa de
esta feature**. En la 003 aprendimos por qué: un peso que arregla un caso rompe otro en silencio, y
sin una tabla que se compruebe entera, la regresión no aparece hasta mucho después.

Viven en `core/tests/fixtures/interpretaciones/` y cada una lleva:

- la canción sobre la que se tocó,
- la lista de observaciones con sus instantes,
- el nivel de exigencia,
- **el resultado esperado, escrito a mano**.

Escrito a mano, no volcado de la implementación: un resultado volcado copia el fallo de quien lo
generó y la prueba pasa a confirmar el error en vez de detectarlo.

## Escenarios de validación

Cada uno corresponde a un criterio de éxito y se ejecuta sin nada enchufado.

### 1. La interpretación perfecta (SC-001)

```sh
cargo test -p piano-core --test evaluacion_test perfecta
```

Una pieza de 20 notas tocadas exactamente en su instante. **Espera**: 20 acertadas, 0 omitidas, 0 de
más, en los **tres** niveles. Si un nivel da otra cosa, las ventanas no están ordenadas por
inclusión y SC-006 está roto.

### 2. No se tocó nada (SC-002)

```sh
cargo test -p piano-core --test evaluacion_test sin_tocar
```

Cero observaciones. **Espera**: se comunica «no se tocó nada», **no** un 0 % de aciertos. Son cosas
distintas y el alumno las lee distinto.

### 3. El retraso sistemático (SC-003, SC-004)

```sh
cargo test -p piano-core --test evaluacion_test sistematico
```

Las 20 notas, todas 40 ms tarde (dentro de tolerancia) y luego todas 120 ms tarde (fuera).
**Espera**: en el primer caso 20 acertadas **y** aviso de desfase; en el segundo, el aviso de
desfase en vez de 20 fallos sueltos.

### 4. La velocidad no regala tolerancia (SC-012)

```sh
cargo test -p piano-core --test evaluacion_test velocidad
```

La misma interpretación con los mismos desfases absolutos, a velocidad normal y a la mitad.
**Espera**: el **mismo** número de aciertos. Si a mitad de velocidad salen más, la tolerancia está
escalando con el tempo y FR-008a está roto.

### 5. Determinismo (SC-005, SC-008)

```sh
cargo test -p piano-core --test evaluacion_test determinismo
```

La misma interpretación evaluada 100 veces, y con las observaciones simultáneas entregadas en
distinto orden. **Espera**: resultados idénticos, siempre.

### 6. El presupuesto (SC-007)

```sh
cargo test -p piano-core --release --test evaluacion_test presupuesto
```

Una pieza de 10 minutos con su interpretación completa. **Espera**: menos de 1 segundo.

### 7. La ruta crítica (Principio IV)

```sh
cargo test -p piano-core --test evaluacion_test coste_por_pulsacion
```

Se **cuenta**, no se cronometra: el número de notas examinadas por pulsación no puede crecer con el
tamaño de la canción. Cronometrar sería intermitente y no demostraría nada estructural, y en la 003
esta forma de medir destapó un coste 30 veces mayor que ninguna prueba de tiempo vio.

### 8. La tabla entera (FR-022)

```sh
cargo test -p piano-core --test evaluacion_test tabla
```

**Todas** las interpretaciones grabadas contra su resultado esperado, de una vez. Existe como tabla
y no como pruebas sueltas por la lección de la 003: un ajuste de tolerancia que arregla un caso
puede romper otro, y solo se ve comprobándolos juntos.

## Comprobación manual

Ninguna es automatizable, y por eso están escritas:

- **SC-010 con una persona**: que alguien toque un pasaje dos veces, una claramente mejor, y
  confirme que el sistema señala la que él considera mejor. El orden léxico es una decisión de
  diseño; que coincida con lo que siente un músico hay que comprobarlo.
- **El tono del resumen**: que un principiante lea su resultado y no se desanime. Un motor correcto
  que desmoraliza es un motor que nadie usará dos veces, y eso no lo detecta ninguna prueba.

## Añadir una interpretación de referencia

Las doce que hay viven en `core/tests/tabla_test.rs` y se comprueban **todas a la vez**. Se
añaden así:

1. Un caso nuevo en `CASOS`, con su canción, lo que el alumno tocó y el nivel.
2. **El resultado esperado, calculado a mano** a partir de la especificación y de los
   umbrales de `tolerancias.rs`. Nunca volcado de la implementación: un resultado volcado
   copia el fallo de quien lo generó, y la prueba pasa a confirmar el error en vez de
   detectarlo.
3. El campo `porque`: qué protege este caso. Un caso que no puede explicarse no aporta nada,
   y con el tiempo nadie se atreve a tocarlo porque nadie sabe qué vigila.

### Cuando un cambio de reglas altera algún resultado

`FR-022` lo dice: hay que **declarar cuáles cambian y por qué** en el pull request. No se
ajusta el número y se sigue — ese es exactamente el momento en que la red deja de serlo.

La prueba lo pone fácil: lista cada discrepancia con el caso, el campo, lo obtenido, lo
esperado y el motivo por el que ese caso existe.

### Verificado que es una red de verdad

No basta con que la tabla pase; hay que haberla visto fallar. Comprobado el 2026-08-19
saboteando dos umbrales:

| Sabotaje | Casos que lo cazaron |
|---|---|
| Ventana intermedia de 60 → 20 ms | «adelanto uniforme», en `acertadas` y `fuera_de_tiempo` |
| Umbral de desfase sistemático de 30 → 60 ms | «retraso uniforme dentro de tolerancia» y «adelanto uniforme», en `desfase` |

## Comprobaciones manuales pendientes

Ninguna de las dos se puede automatizar, y por eso están escritas: si no están planificadas,
nadie las hace.

### SC-010 con una persona (T080)

Que alguien toque un pasaje **dos veces**, una claramente mejor que la otra, y confirme que
el sistema señala como mejor la que él considera mejor.

El orden es léxico —manda el número de notas y el ritmo solo desempata— y eso es una
**decisión de diseño**. Que coincida con lo que siente un músico hay que comprobarlo con un
músico; ninguna prueba puede hacerlo.

Si no coincide, la conversación no es sobre el código sino sobre `FR-020`, y el sitio donde
se resuelve es `/speckit-clarify`, no un ajuste silencioso.

### El tono del resumen (T081)

Que un principiante lea su resultado y **no se desanime**.

Un motor correcto que desmoraliza es un motor que nadie usa dos veces, y eso no lo detecta
ninguna prueba. Cosas concretas que mirar:

- ¿«3 de 20» se lee como un suspenso o como un punto de partida?
- Cuando hay desfase sistemático, ¿el mensaje suena a diagnóstico útil o a reproche?
- ¿Se entiende que las notas fuera del alcance del teclado y los pasajes saltados **no**
  cuentan en su contra?

## Medido en esta feature

| Qué | Cuándo | Resultado |
|---|---|---|
| Pérdida por no mirar al futuro (T074) | 2026-08-19 | **1 emparejamiento** en el peor caso de seis; ver `research.md` |
| Coste por pulsación (T075) | 2026-08-19 | No crece con la pieza tras añadir el cursor monótono. Antes: 1.300 notas examinadas con 200 y 32.968 con 4.000 |
| Presupuesto de evaluación (T076) | 2026-08-19 | Diez minutos y 6.000 notas, muy por debajo del segundo de SC-007 |
