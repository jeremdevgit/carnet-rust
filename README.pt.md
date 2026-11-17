<p align="center"><img src="assets/carnet-de-notes.svg" width="112" alt=""></p>

<h1 align="center">Carnet de notes</h1>

<p align="center"><b>Cadernos encriptados para Windows, com Markdown, tarefas, histórico e sincronização experimental com Nextcloud, WebDAV e Joplin.</b></p>

<p align="center"><img src="https://img.shields.io/badge/%F0%9F%93%92%20Productivity%20%26%20data-Windows-1a9a6c?style=flat-square" alt="Produtividade e dados"> <a href="https://github.com/jeremdevgit/carnet-rust/releases"><img src="https://img.shields.io/github/v/release/jeremdevgit/carnet-rust?include_prereleases&style=flat-square&label=version" alt="version"></a> <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-555?style=flat-square" alt="GPL-3.0-or-later"></a> <a href="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml"><img src="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg" alt="CI"></a></p>

<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.de.md">Deutsch</a> · <a href="README.it.md">Italiano</a> · <b>Português</b> · <a href="README.zh.md">简体中文</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><a href="https://github.com/jeremdevgit/carnet-rust/releases/latest"><b>⬇ Transferir</b></a></p>

![Carnet de notes, tema escuro com notas de demonstração](screenshots/en/liquid-glass-dark.png)

O Carnet de notes é uma aplicação de cadernos nativa para Windows, com uma interface clara/escura inspirada no Liquid Glass. **As suas notas estão sempre encriptadas no disco** e pode sincronizá-las com Nextcloud, WebDAV ou Joplin Cloud/Server. A paridade total com o Joplin ainda não foi atingida — consulte [COMPATIBILITY.md](COMPATIBILITY.md).

<p><img src="screenshots/en/liquid-glass-light.png" alt="Tema claro" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Configuração do cofre encriptado" width="49%"></p>

## Em resumo

| | |
|---|---|
| **Categoria** | 📒 Produtividade e dados |
| **Plataformas** | Windows |
| **Interface** | 8 idiomas (EN, FR, ES, DE, IT, PT, ZH, JA) |
| **Preço** | Gratuito, de código aberto, sem anúncios nem telemetria |
| **Estado** | Estável |
| **Licença** | [GPL-3.0-or-later](LICENSE) |

## Funções

- **Cadernos** em árvore (mudar o nome, mover, eliminação segura), edição e pré-visualização Markdown, ligações internas, duplicação.
- **Tarefas** com prazos, **etiquetas** (mudar o nome, fundir, remover), pesquisa de texto, listas paginadas.
- **Gravação automática** a cada 800 ms, Ctrl+S / Ctrl+N, 100 versões por nota, reciclagem com recuperação.
- **Pré-visualização de imagens** locais (PNG/JPEG); imagens remotas e ligações desconhecidas bloqueadas.
- **Importar/exportar**: Markdown UTF-8 e JEX experimental (exportações não encriptadas, com aviso).
- Tipo de letra CJK incluído para mostrar corretamente chinês e japonês.

## Os seus dados

- **Sempre encriptados**: notas, anexos, histórico, rascunhos, índice de pesquisa, definições e páginas WAL são guardados com **SQLCipher**.
- **Dois modos de proteção**: por omissão, uma chave aleatória guardada no **Gestor de Credenciais do Windows** (sem palavra-passe extra, funciona offline); ou uma **palavra-passe mestra** opcional (12 caracteres ou mais, sem porta traseira, bloqueio após 5 minutos de inatividade por omissão).
- **Cópias de segurança encriptadas** com palavra-passe própria, transportáveis para outro PC; o restauro guarda antes uma cópia de recuperação encriptada.
- Pasta do perfil: `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`. A desinstalação mantém-na.

## Sincronização (experimental)

| Serviço | O que precisa |
|---|---|
| **Nextcloud** | Endereço HTTPS, nome de utilizador, uma *palavra-passe de aplicação* e uma pasta privada `Joplin` |
| **WebDAV** | O URL HTTPS completo de uma pasta dedicada às notas |
| **Joplin Cloud / Server** | A predefinição Cloud ou o endereço da API do seu servidor e as credenciais (MFA/SSO interativos não suportados) |

A sincronização corre ao abrir e a cada cinco minutos enquanto estiver desbloqueado; as notas continuam editáveis offline. **A encriptação ponto a ponto do Joplin é suportada** nos dois sentidos (verificada com o Joplin CLI 3.7.1). Os conflitos pedem uma escolha explícita e um lote que apague 90% das notas é travado por segurança. HTTP só é permitido localmente, os redirecionamentos são recusados e a verificação TLS mantém-se ativa. Sem E2EE, o servidor pode ler o conteúdo sincronizado.

## Instalar

Transfira `CarnetDeNotes-<versão>-Windows-x64-Setup.exe` da [versão mais recente](https://github.com/jeremdevgit/carnet-rust/releases/latest) e execute-o — por utilizador, sem direitos de administrador. Precisa de um controlador OpenGL; no Windows 11 usa Mica. O executável não está assinado, por isso o SmartScreen pode pedir confirmação. Os perfis de versões anteriores abrem tal como estão.

## Compilar

```powershell
# Rust 1.95+, Microsoft C++ Build Tools, Perl (OpenSSL incluído)
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/package.ps1` cria o instalador com o Inno Setup 6. A CI compila-o, testa-o, instala-o e desinstala-o no Windows. Detalhes: [VALIDATION.md](VALIDATION.md).

**Ainda não implementado**: plugins, partilha de cadernos, Web Clipper, texto formatado, OCR, desenhos, importação ENEX/OneNote, exportação HTML/PDF, lembretes.

## Licença

GPL-3.0-or-later — ver [LICENSE](LICENSE). Implementação independente sem código, logótipo nem ícone do Joplin; sem afiliação ao Joplin. As bibliotecas e o tipo de letra incluídos mantêm as suas licenças ([THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)).

---

Encontrou uma vulnerabilidade? Comunique-a em privado (consulte [SECURITY.md](SECURITY.md)) e não num issue público.

## Mais software livre de jeremdevgit

Todos gratuitos, de código aberto e sem anúncios.

**🛠️ Sistema e hardware** — [Boîte à outils](https://github.com/jeremdevgit/boite-a-outils) · [rJb](https://github.com/jeremdevgit/rjb) · [LJ Hub](https://github.com/jeremdevgit/lj-hub) · [Decklight](https://github.com/jeremdevgit/decklight)

**📒 Produtividade e dados** — [JMeca](https://github.com/jeremdevgit/jmeca) · [Suite Libre](https://jeremdevgit.github.io/suite-libre/)

**🎮 Música e jogos** — [Pomme](https://github.com/jeremdevgit/pomme) · [Ludothèque](https://github.com/jeremdevgit/ludotheque) · [AutoDex](https://github.com/jeremdevgit/autodex) · [Batailles de France](https://github.com/jeremdevgit/batailles-de-france)
