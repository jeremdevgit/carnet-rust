[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

# Carnet Rust 0.3.0

Aplicación nativa Windows con tema Liquid Glass y ocho idiomas. Incluye copiar/cortar/pegar texto desde el portapapeles del sistema. Conexión genérica con Nextcloud, WebDAV o Joplin Cloud/Server, sin servidor privado predefinido.

👉 **[Descargar Carnet Rust para Windows](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetRust-0.3.0-Windows-x64.zip`, aplicación portátil)

![Tema oscuro de Carnet Rust con notas de demostración](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Tema claro con notas de demostración" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Configuración del almacén cifrado con una cuenta Nextcloud genérica" width="49%"></p>

*Capturas de la interfaz real con datos de demostración, tomadas en un entorno de prueba sin pantalla.*

Extrae el ZIP y abre CarnetRust.exe. Por defecto, los datos locales se cifran con SQLCipher y una clave aleatoria guardada en el Administrador de credenciales de Windows. El uso sin conexión no requiere una segunda contraseña. La protección depende de la sesión del sistema; no exige Windows Hello al abrir. Bloquea Windows al dejar el equipo. El bloqueo automático está desactivado en este modo.

La opción de contraseña maestra sigue disponible (mínimo 12 caracteres, sin recuperación). Con ella el bloqueo automático es de 5 minutos; se puede configurar de 0 a 60, donde 0 lo desactiva. Los perfiles 0.2 conservan su contraseña actual y pueden cambiar de modo desde Configuración. Los perfiles locales 0.1 se migran cifrados; no se modifica el perfil oficial Joplin ni se borran físicamente bloques antiguos.

Para Nextcloud, introduce la dirección HTTPS de tu servidor, el usuario y una contraseña de aplicación. Crea previamente la carpeta Joplin en los archivos privados de esa cuenta. WebDAV acepta la URL HTTPS completa de una carpeta dedicada. Joplin Cloud/Server usa su API; no se ha probado una cuenta Cloud real ni MFA/SSO.

La sincronización automática se inicia al abrir y reintenta cada 5 minutos mientras el almacén esté abierto. Las notas siguen disponibles sin red/VPN. Los cambios de cuenta requieren Guardar o Conectar y sincronizar. Las credenciales permanecen dentro del almacén cifrado. Pruebas aisladas: Nextcloud 35.0.2 y Joplin Server 3.7.1, conflictos, adjuntos y eliminaciones. La compatibilidad es experimental.

**El cifrado local no es E2EE Joplin. Los destinos E2EE, cuadernos compartidos y notas HTML todavía se rechazan.** El servidor puede leer las notas sincronizadas si E2EE está desactivado. No se desactiva nunca el cifrado de un destino existente.

Las copias portátiles piden una contraseña propia para poder restaurarlas en otro dispositivo. Copiar únicamente el perfil no copia la clave del sistema. Conservar las contraseñas de copias anteriores. Las copias automáticas no se purgan; al pasar de clave del sistema a contraseña maestra, se crea una copia bajo la nueva contraseña. Los antiguos instantáneos bajo la clave retirada no usan esa nueva contraseña.

Notas Markdown, cuadernos, tareas, etiquetas, búsqueda, historial, papelera, adjuntos y vistas previas PNG/JPEG. Exportaciones Markdown/JEX y adjuntos exportados en claro. Faltan E2EE, editor enriquecido, extensiones, OCR y otras funciones: COMPATIBILITY.md.

El ejecutable fue compilado para Windows, sin firma; ejecución nativa y Gestor de credenciales de Windows todavía pendientes de verificación real. Sin mediciones comparativas de RAM/CPU ni certificación de seguridad. Véanse SECURITY.md y VALIDATION.md.

Compilar: Rust 1.95+, herramientas C++ y Perl; cargo test --locked, cargo clippy --locked --all-targets -- -D warnings, cargo build --locked --release.

Código original GPL-3.0-or-later. Proyecto independiente, sin afiliación con Joplin. Autor público: [jeremdevgit](https://github.com/jeremdevgit). Licencias en THIRD-PARTY-NOTICES.txt y assets/.
