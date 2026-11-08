[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

# Carnet Rust 0.3.0

Aplicativo nativo de notas para Windows, criado por **jeremdevgit**, com tema claro e escuro inspirado em Liquid Glass e oito idiomas. Copiar, recortar e colar usam a área de transferência do sistema. Há conexão genérica com Nextcloud, WebDAV e Joplin Cloud/Server, sem servidor privado predefinido. Os dados locais são criptografados com SQLCipher. **A paridade completa com o Joplin ainda não foi alcançada**; veja [COMPATIBILITY.md](COMPATIBILITY.md).

👉 **[Baixar Carnet Rust para Windows](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetRust-0.3.0-Windows-x64.zip`, aplicativo portátil)

![Tema escuro com notas de demonstração](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Tema claro com notas de demonstração" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Configuração do cofre criptografado" width="49%"></p>

*As imagens mostram a interface real com dados de demonstração, capturada em um ambiente de teste sem tela.*

## Instalação e proteção local

Extraia o ZIP para Windows e execute `CarnetRust.exe`. O uso local não exige conta nem privilégios de administrador. O executável não é assinado e precisa de um driver OpenGL funcional. No Windows 11, o aplicativo solicita Mica; caso contrário, gradientes e painéis translúcidos continuam disponíveis.

Por padrão, uma chave aleatória do SQLCipher é guardada no **Gerenciador de Credenciais do Windows**. As notas continuam acessíveis sem internet e sem uma segunda senha. Essa proteção depende da segurança da conta do Windows; o Windows Hello não é solicitado a cada abertura. Bloqueie a sessão do Windows quando se afastar. A senha da conta de sincronização não é a chave do banco de dados.

A **senha mestra** é opcional e precisa ter pelo menos 12 caracteres. Não há recuperação dessa senha. Nesse modo, o bloqueio automático ocorre após cinco minutos por padrão; com a chave do sistema, ele fica desativado por padrão. É possível mudar de modo nas configurações. Perfis da versão 0.2 mantêm a senha anterior. Perfis locais 0.1 sem criptografia são migrados, mas cópias externas antigas e blocos do SSD não são criptografados retroativamente. O perfil oficial do Joplin não é alterado.

Backups portáteis criptografados exigem uma **senha própria de backup**. Copiar apenas a pasta do perfil não transfere a chave guardada pelo Windows; para outro computador, use um backup portátil ou sincronize novamente. Se a credencial do sistema for perdida, o cache local pode ficar inacessível.

## Recursos e sincronização

Há cadernos e subcadernos, notas Markdown com prévia, tarefas com vencimento, etiquetas, pesquisa, histórico, lixeira, anexos e prévias locais de PNG/JPEG. As alterações são salvas automaticamente. Há importação e exportação de Markdown e JEX experimental; os arquivos exportados ficam **sem criptografia**.

- **Nextcloud:** informe o endereço HTTPS do servidor, usuário e senha de aplicativo. Antes, crie uma pasta privada `Joplin` nos arquivos dessa conta. O aplicativo monta o endereço WebDAV; também aceita um URL completo `/remote.php/`.
- **WebDAV:** use o URL HTTPS completo de uma pasta exclusiva para notas, não a raiz do servidor ou da conta.
- **Joplin Cloud/Server:** use a configuração Cloud ou o endereço da API do seu servidor e suas credenciais. Login interativo Cloud, MFA e SSO não são suportados; nenhuma conta Cloud real foi testada.

Em contas novas, a sincronização automática começa após a abertura e tenta novamente a cada cinco minutos enquanto o cofre estiver aberto. As notas locais continuam editáveis sem rede ou VPN. As credenciais ficam no cofre criptografado. Testes isolados com Nextcloud 35.0.2 e Joplin Server 3.7.1 passaram; a compatibilidade geral ainda é experimental.

**A criptografia local SQLCipher não é a criptografia de ponta a ponta (E2EE) do Joplin. Destinos E2EE, cadernos compartilhados e notas HTML são recusados sem desativar a criptografia existente.** Sem E2EE, o servidor pode ler o conteúdo sincronizado.

## Estado e código-fonte

O executável Windows foi compilado, mas não executado em um PC Windows real. Não há comparação confiável de RAM/CPU com o Joplin nem certificação de segurança. E2EE, plugins, Web Clipper, editor de texto avançado, OCR e outros recursos ainda faltam. Veja [VALIDATION.md](VALIDATION.md) e [SECURITY.md](SECURITY.md).

Para compilar: Rust 1.95+, ferramentas C++ da Microsoft e Perl; `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo build --locked --release`. Código original GPL-3.0-or-later. Projeto independente, sem afiliação com o Joplin. Licenças das dependências em [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). Autor público: [jeremdevgit](https://github.com/jeremdevgit).
