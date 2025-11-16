# Lexum Installation Scripts

Scripts de instalação para Lexum que baixam e instalam diretamente do repositório GitHub, sem necessidade de domínio customizado.

## Linux / macOS

### Instalação rápida (recomendado)

```bash
curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/install.sh | bash
```

### Ou baixe e execute manualmente

```bash
curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/scripts/install.sh -o install.sh
chmod +x install.sh
sudo ./install.sh
```

## Windows

### Instalação rápida (recomendado)

```powershell
powershell -c "irm https://raw.githubusercontent.com/hivellm/lexum/main/install.ps1 | iex"
```

### Ou baixe e execute manualmente

```powershell
# Baixar script
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/hivellm/lexum/main/scripts/install.ps1" -OutFile "install.ps1"

# Executar como Administrador
.\install.ps1
```

## O que os scripts fazem

### Linux / macOS

1. **Instala dependências**: Git, Rust, build tools, SSL libraries
2. **Clona o repositório**: Baixa o código do GitHub
3. **Compila o projeto**: Usa `cargo build --release`
4. **Instala binários**: Copia para `/opt/lexum/bin` e cria symlink em `/usr/local/bin/lexum`
5. **Configura serviço systemd**: Cria serviço que inicia automaticamente no boot
6. **Inicia o serviço**: Lexum fica rodando como serviço

### Windows

1. **Instala dependências**: Git, Rust (via rustup)
2. **Clona o repositório**: Baixa o código do GitHub
3. **Compila o projeto**: Usa `cargo build --release`
4. **Instala binários**: Copia para `C:\Program Files\Lexum\bin`
5. **Adiciona ao PATH**: CLI fica disponível em qualquer terminal
6. **Configura Windows Service**: Usa NSSM para criar serviço que inicia automaticamente
7. **Inicia o serviço**: Lexum fica rodando como serviço

## Variáveis de ambiente (opcional)

### Linux

```bash
# Personalizar diretórios de instalação
export LEXUM_INSTALL_DIR="/custom/path/lexum"
export LEXUM_DATA_DIR="/custom/path/data"
export LEXUM_CONFIG_DIR="/custom/path/config"
export LEXUM_USER="custom-user"

curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/install.sh | bash
```

### Windows

```powershell
# Personalizar diretórios de instalação
$env:LEXUM_INSTALL_DIR = "D:\Lexum"
$env:LEXUM_DATA_DIR = "D:\LexumData"

powershell -c "irm https://raw.githubusercontent.com/hivellm/lexum/main/install.ps1 | iex"
```

## Pós-instalação

### Linux

```bash
# Verificar status do serviço
sudo systemctl status lexum

# Ver logs
sudo journalctl -u lexum -f

# Reiniciar serviço
sudo systemctl restart lexum

# Parar serviço
sudo systemctl stop lexum

# Testar CLI
lexum --help
```

### Windows

```powershell
# Verificar status do serviço
Get-Service Lexum

# Ver logs
Get-Content "C:\ProgramData\Lexum\logs\service.log" -Tail 50 -Wait

# Reiniciar serviço
Restart-Service Lexum

# Parar serviço
Stop-Service Lexum

# Testar CLI
lexum-server --help
```

## Desinstalação

### Linux

```bash
# Parar e desabilitar serviço
sudo systemctl stop lexum
sudo systemctl disable lexum

# Remover arquivos
sudo rm -rf /opt/lexum
sudo rm -rf /var/lib/lexum
sudo rm -rf /etc/lexum
sudo rm /usr/local/bin/lexum
sudo rm /etc/systemd/system/lexum.service

# Recarregar systemd
sudo systemctl daemon-reload

# Remover usuário (opcional)
sudo userdel lexum
```

### Windows

```powershell
# Parar e remover serviço
Stop-Service Lexum
& "C:\Program Files\Lexum\bin\nssm.exe" remove Lexum confirm

# Remover arquivos
Remove-Item -Recurse -Force "C:\Program Files\Lexum"
Remove-Item -Recurse -Force "C:\ProgramData\Lexum"

# Remover do PATH (manual)
# Editar variáveis de ambiente e remover C:\Program Files\Lexum\bin
```

## Requisitos

### Linux / macOS

- Git
- Rust (instalado automaticamente se não presente)
- Build tools (gcc, make, pkg-config)
- OpenSSL development libraries
- sudo/root access para instalação do serviço

### Windows

- Git (instalado automaticamente se não presente)
- Rust (instalado automaticamente se não presente)
- PowerShell 5.1+ (Administrator)
- Windows 10/11 ou Windows Server 2016+

## Troubleshooting

### Linux: Serviço não inicia

```bash
# Verificar logs
sudo journalctl -u lexum -n 50

# Verificar permissões
sudo chown -R lexum:lexum /opt/lexum /var/lib/lexum /etc/lexum

# Verificar configuração
sudo systemctl cat lexum
```

### Windows: Serviço não inicia

```powershell
# Verificar logs
Get-Content "C:\ProgramData\Lexum\logs\service-error.log"

# Verificar configuração do serviço
& "C:\Program Files\Lexum\bin\nssm.exe" get Lexum AppParameters

# Verificar permissões
icacls "C:\Program Files\Lexum" /grant "NT AUTHORITY\SYSTEM:(OI)(CI)F"
```

### Build falha

- Verifique se tem espaço em disco suficiente (pelo menos 2GB livres)
- Verifique conexão com internet (para baixar dependências do Cargo)
- No Linux, certifique-se de que todas as dependências de desenvolvimento estão instaladas

## Segurança

Os scripts baixam código diretamente do repositório GitHub público. Sempre revise o código antes de executar:

```bash
# Ver o script antes de executar
curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/scripts/install.sh
```

## Suporte

Para problemas ou questões:
- Issues: https://github.com/hivellm/lexum/issues
- Documentação: https://github.com/hivellm/lexum/tree/main/docs

