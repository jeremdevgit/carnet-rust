<p align="center"><img src="assets/carnet-de-notes.svg" alt="Carnet de notes" width="128"></p>

# Carnet de notes

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)


Aplicación nativa Windows con tema Liquid Glass y ocho idiomas. Incluye copiar/cortar/pegar texto desde el portapapeles del sistema. Conexión genérica con Nextcloud, WebDAV o Joplin Cloud/Server, sin servidor privado predefinido.

👉 **[Descargar Carnet de notes para Windows](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetDeNotes-0.5.0-Windows-x64-Setup.exe`, instalador)

![Tema oscuro de Carnet de notes con notas de demostración](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Tema claro con notas de demostración" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Configuración del almacén cifrado con una cuenta Nextcloud genérica" width="49%"></p>

*Capturas de la interfaz real con datos de demostración, tomadas en un entorno de prueba sin pantalla.*

Ejecuta el instalador (por usuario, sin derechos de administrador) y abre Carnet de notes desde el menú Inicio; desinstalar conserva tus cuadernos cifrados. Por defecto, los datos locales se cifran con SQLCipher y una clave aleatoria guardada en el Administrador de credenciales de Windows. El uso sin conexión no requiere una segunda contraseña. La protección depende de la sesión del sistema; no exige Windows Hello al abrir. Bloquea Windows al dejar el equipo. El bloqueo automático está desactivado en este modo.

Los perfiles 0.3 permanecen en la misma carpeta cifrada y conservan la misma entrada del Administrador de credenciales de Windows. El nuevo ejecutable los abre sin copiar ni convertir el perfil.

La opción de contraseña maestra sigue disponible (mínimo 12 caracteres, sin recuperación). Con ella el bloqueo automático es de 5 minutos; se puede configurar de 0 a 60, donde 0 lo desactiva. Los perfiles 0.2 conservan su contraseña actual y pueden cambiar de modo desde Configuración. Los perfiles locales 0.1 se migran cifrados; no se modifica el perfil oficial Joplin ni se borran físicamente bloques antiguos.

Para Nextcloud, introduce la dirección HTTPS de tu servidor, el usuario y una contraseña de aplicación. Crea previamente la carpeta Joplin en los archivos privados de esa cuenta. WebDAV acepta la URL HTTPS completa de una carpeta dedicada. Joplin Cloud/Server usa su API; no se ha probado una cuenta Cloud real ni MFA/SSO.

La sincronización automática se inicia al abrir y reintenta cada 5 minutos mientras el almacén esté abierto. Las notas siguen disponibles sin red/VPN. Los cambios de cuenta requieren Guardar o Conectar y sincronizar. Las credenciales permanecen dentro del almacén cifrado. Pruebas aisladas: Nextcloud 35.0.2 y Joplin Server 3.7.1, conflictos, adjuntos y eliminaciones. La compatibilidad es experimental.

**El cifrado de extremo a extremo (E2EE) de Joplin está soportado**: indica la contraseña maestra en los ajustes de sincronización; verificado en ambos sentidos con el cliente oficial Joplin CLI 3.7.1. Las notas HTML se conservan y se muestran como texto. Compartir cuadernos aún no está disponible. El servidor puede leer las notas sincronizadas si E2EE está desactivado. No se desactiva nunca el cifrado de un destino existente.

Las copias portátiles piden una contraseña propia para poder restaurarlas en otro dispositivo. Copiar únicamente el perfil no copia la clave del sistema. Conservar las contraseñas de copias anteriores. Las copias automáticas no se purgan; al pasar de clave del sistema a contraseña maestra, se crea una copia bajo la nueva contraseña. Los antiguos instantáneos bajo la clave retirada no usan esa nueva contraseña.

Notas Markdown, cuadernos, tareas, etiquetas, búsqueda, historial, papelera, adjuntos y vistas previas PNG/JPEG. Exportaciones Markdown/JEX y adjuntos exportados en claro. Faltan compartir cuadernos, editor enriquecido, extensiones, OCR y otras funciones: COMPATIBILITY.md.

El ejecutable fue compilado para Windows, sin firma; ejecución nativa y Gestor de credenciales de Windows todavía pendientes de verificación real. Sin mediciones comparativas de RAM/CPU ni certificación de seguridad. Véanse SECURITY.md y VALIDATION.md.

Compilar: Rust 1.95+, herramientas C++ y Perl; cargo test --locked, cargo clippy --locked --all-targets -- -D warnings, cargo build --locked --release.

Código original GPL-3.0-or-later. Proyecto independiente, sin afiliación con Joplin. Autor público: [jeremdevgit](https://github.com/jeremdevgit). Licencias en THIRD-PARTY-NOTICES.txt y assets/.

## Licencia



## Mis otros programas

Todos libres, gratuitos y sin publicidad — [github.com/jeremdevgit](https://github.com/jeremdevgit):

| Programa | Plataforma | En resumen |
|---|---|---|
| [AutoDex](https://github.com/jeremdevgit/autodex) | Windows, macOS, Linux | Juego de colección de coches con reconocimiento de fotos en el dispositivo |
| [Boîte à outils](https://github.com/jeremdevgit/boite-a-outils) | Windows, macOS | Mantenimiento del ordenador, limpieza y búsqueda de archivos |
| [Decklight](https://github.com/jeremdevgit/decklight) | Windows, macOS, Linux | Stream Deck virtual con control USB |
| [JMeca](https://github.com/jeremdevgit/jmeca) | Windows, macOS, Linux | Seguimiento de mantenimiento de vehículos cifrado |
| [LJ Hub](https://github.com/jeremdevgit/lj-hub) | Windows, macOS, Linux | Gestor de perfiles para dispositivos Logitech |
| [Pomme](https://github.com/jeremdevgit/pomme) | Windows | Reproductor de música para Apple Music y archivos locales |
| [rJb](https://github.com/jeremdevgit/rjb) | Windows | Control de la iluminación RGB |
| [Ludothèque](https://github.com/jeremdevgit/ludotheque) | macOS | Biblioteca y lanzador de juegos |
| [Suite Libre](https://jeremdevgit.github.io/suite-libre/) | Navegador | Seis herramientas sin cuenta |
| [Batailles de France](https://github.com/jeremdevgit/batailles-de-france) | Navegador | Juego de estrategia en tiempo real |
