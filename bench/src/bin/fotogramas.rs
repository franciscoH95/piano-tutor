//! El banco de fotogramas **no vive aqui**, y conviene decir por que.
//!
//! SC-003 mide el ritmo al que la pantalla muestra los fotogramas. La propia
//! especificacion lo deja escrito: *«esta medido que sin una ventana visible en pantalla el
//! sistema no dibuja ni un fotograma»*. La medicion tiene que ocurrir **dentro de la
//! ventana real**, con las marcas de tiempo que da `requestAnimationFrame`; un binario de
//! Rust sin ventana no puede observar nada de eso.
//!
//! Lo que si se puede probar —y es donde esta el riesgo— es el **calculo**. SC-003c avisa
//! de que un informe que ignore las suspensiones del sistema publica un numero inventado:
//! en la primera medicion se perdieron 430 de 600 segundos por esa causa. Eso es logica, no
//! instrumentacion, y vive en `src/practica/fotogramas.ts`, con 13 pruebas que incluyen
//! trazas con suspensiones, trazas degeneradas y la distincion entre el coste de dibujar y
//! el ritmo de la pantalla.
//!
//! El procedimiento para tomar la medida esta en
//! `specs/003-practicar-una-cancion/quickstart.md`.

fn main() {
    println!("El banco de fotogramas necesita una ventana visible.");
    println!("Analisis:      src/practica/fotogramas.ts");
    println!("Procedimiento: specs/003-practicar-una-cancion/quickstart.md");
}
