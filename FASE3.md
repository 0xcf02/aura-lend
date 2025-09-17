# 🚀 FASE 3: Setup Devnet e Deploy Automático

## 📊 Resumo da Situação Atual
- ✅ **FASE 1 COMPLETA**: GitHub Actions funcionando sem emails de erro
- ✅ **FASE 2 COMPLETA**: Qualidade de código (cargo fmt + npm lint) integrada ao CI
- 🎯 **FASE 3 OBJETIVO**: Configurar deploy automático para Solana Devnet

## 🔍 Arquivos Analisados
- **Anchor.toml**: Configurado para devnet com Program ID placeholder `AuraLendVxVxVxVxVxVxVxVxVxVxVxVxVxVxVxVxVxVxV`
- **package.json**: Scripts de deploy já existem (`deploy`, `verify`, `deploy:init:devnet`)
- **scripts/upgrade/deploy.ts**: Sistema completo de deploy com multisig e upgradeability
- **.github/workflows/main.yml**: CI atual sem deploy automático

---

## ❓ PERGUNTAS QUE VOCÊ PRECISA RESPONDER

### 1. **Conta Solana Devnet**
- [ ] Você tem uma carteira Solana configurada?
- [ ] Caminho da carteira: `~/.config/solana/id.json` ou outro local?
- [ ] Tem SOL suficiente na devnet para deploy? (precisa ~2-5 SOL)
- [ ] Solana CLI já está instalado na sua máquina?

### 2. **Estratégia de Deploy**
- [ ] Quer deploy automático em cada push na branch `main`?
- [ ] Ou prefere deploy manual via GitHub Actions `workflow_dispatch`?
- [ ] Quer que o deploy rode apenas quando há mudanças no código Rust (`programs/`)?
- [ ] Quer deploy em branches de feature também ou só na main?

### 3. **Program ID e Segurança**
- [ ] Precisa gerar novos keypairs para o programa?
- [ ] Como quer gerenciar as chaves privadas no GitHub (via Secrets)?
- [ ] Quer usar o mesmo Program ID para devnet e mainnet ou diferentes?
- [ ] Tem preferência por algum Program ID específico?

---

## 🛠️ COMANDOS DE PREPARAÇÃO MANUAL

Execute estes comandos antes de prosseguir com a automação:

### 1. Instalar Solana CLI (se não tiver)
```bash
sh -c "$(curl -sSfL https://release.solana.com/v1.18.22/install)"
echo 'export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 2. Configurar para Devnet
```bash
# Configurar URL da devnet
solana config set --url https://api.devnet.solana.com

# Verificar configuração
solana config get
```

### 3. Gerar/Configurar Carteira (se não tiver)
```bash
# Gerar nova carteira
solana-keygen new --outfile ~/.config/solana/id.json

# OU usar carteira existente
# solana config set --keypair /path/to/your/keypair.json

# Verificar carteira
solana address
```

### 4. Conseguir SOL na Devnet
```bash
# Airdrop SOL (máximo 2 SOL por vez)
solana airdrop 2

# Verificar saldo
solana balance

# Se precisar de mais SOL, repita o airdrop algumas vezes
```

### 5. Gerar Program Keypair
```bash
# Gerar keypair para o programa
solana-keygen new --outfile program-keypair.json --no-bip39-passphrase

# Ver o Program ID gerado
solana address -k program-keypair.json
```

### 6. Testar Build Local
```bash
# Testar se tudo compila
anchor build

# Verificar se IDL foi gerado
ls -la target/idl/
```

---

## 🔧 CHECKLIST DE IMPLEMENTAÇÃO AUTOMÁTICA

### 3.1 Configurar GitHub Secrets
- [ ] `SOLANA_PRIVATE_KEY`: Chave privada da carteira de deploy (base58)
- [ ] `PROGRAM_KEYPAIR`: Chave privada do programa (JSON array)
- [ ] `DEVNET_RPC_URL`: URL RPC opcional (padrão: https://api.devnet.solana.com)
- [ ] Opcional: `ANCHOR_WALLET`: Path da carteira se diferente do padrão

### 3.2 Criar Workflow de Deploy Devnet
Criar arquivo `.github/workflows/deploy-devnet.yml`:
- [ ] Trigger: push na main + mudanças em `programs/`
- [ ] Setup Solana CLI e Anchor
- [ ] Build do programa Rust
- [ ] Deploy para devnet usando keypairs dos secrets
- [ ] Verificação pós-deploy
- [ ] Update automático do Program ID no código
- [ ] Logs detalhados e notificações

### 3.3 Atualizar Configurações
- [ ] **Anchor.toml**: Substituir placeholder pelo Program ID real
- [ ] **package.json**: Adicionar scripts específicos para devnet
- [ ] **programs/aura-lend/lib.rs**: Verificar se Program ID está correto

### 3.4 Scripts Auxiliares
- [ ] `scripts/deploy/initialize-market.ts`: Inicializar mercado pós-deploy
- [ ] `scripts/deploy/verify-deployment.ts`: Verificação completa
- [ ] `scripts/deploy/setup-reserves.ts`: Configurar reserves iniciais
- [ ] `scripts/deploy/update-program-id.ts`: Atualizar IDs no código

### 3.5 Integração com CI Principal
- [ ] Modificar `.github/workflows/main.yml` para incluir deploy
- [ ] Condicionais para deploy apenas quando tests passam
- [ ] Paralelização: deploy em paralelo com outros jobs
- [ ] Fallback: continue mesmo se deploy falhar (para PRs)

---

## 🎯 RESULTADO FINAL ESPERADO

Após implementação completa:

### ✅ Deploy Automático Funcionando
- Push na main → Tests passam → Deploy automático na devnet
- Program ID real configurado em todos os arquivos
- Mercado inicializado automaticamente pós-deploy

### ✅ Monitoramento e Logs
- Logs detalhados no GitHub Actions
- Verificação pós-deploy automática
- Notificações de sucesso/falha

### ✅ Sistema Escalável
- Fácil adaptação para mainnet posteriormente
- Deploy manual via workflow_dispatch disponível
- Rollback capabilities através do sistema de upgrade

### ✅ Segurança
- Chaves privadas seguras nos GitHub Secrets
- Deploy apenas quando todos os tests passam
- Verificação de integridade pós-deploy

---

## 📁 ARQUIVOS QUE SERÃO CRIADOS/MODIFICADOS

### Novos Arquivos
```
.github/workflows/deploy-devnet.yml    # Workflow de deploy
scripts/deploy/                       # Scripts de deploy
├── initialize-market.ts
├── verify-deployment.ts
├── setup-reserves.ts
└── update-program-id.ts
scripts/deploy/config/                # Configurações
├── devnet.json
└── mainnet.json
```

### Arquivos Modificados
```
Anchor.toml                           # Program ID real
package.json                          # Novos scripts
programs/aura-lend/src/lib.rs        # Program ID declaration
.github/workflows/main.yml           # Integração com deploy
```

---

## 🔄 PARA RETOMAR POSTERIORMENTE

### Como Usar Este Arquivo
1. **Responda as perguntas** na seção "PERGUNTAS QUE VOCÊ PRECISA RESPONDER"
2. **Execute os comandos** na seção "COMANDOS DE PREPARAÇÃO MANUAL"
3. **Confirme que tem SOL** na devnet (`solana balance`)
4. **Forneça as informações** solicitadas (Program ID gerado, path da carteira, etc.)
5. **Dê green light** para começar a implementação automática

### Status Atual
- [ ] Perguntas respondidas
- [ ] Comandos manuais executados
- [ ] SOL disponível na devnet
- [ ] Program keypair gerado
- [ ] Pronto para automação

### Próxima Sessão
Na próxima sessão, me forneça:
1. **Suas respostas** às 3 perguntas principais
2. **Program ID gerado** pelo comando `solana address -k program-keypair.json`
3. **Confirmação** de que tem SOL na devnet
4. **Path da sua carteira** se diferente do padrão

### Comando de Verificação Rápida
```bash
# Execute este comando para verificar se está tudo pronto:
echo "=== STATUS FASE 3 ==="
echo "Solana CLI: $(solana --version)"
echo "Network: $(solana config get | grep 'RPC URL')"
echo "Wallet: $(solana address)"
echo "Balance: $(solana balance)"
echo "Program ID: $(solana address -k program-keypair.json 2>/dev/null || echo 'Not generated yet')"
echo "==================="
```

---

**⚡ Status**: Aguardando suas respostas e preparação manual para prosseguir com a implementação automática.

**📅 Criado**: 2025-01-17
**👤 Responsável**: Usuário (preparação) + Claude (implementação)
**🔗 Repositório**: https://github.com/0xcf02/aura-lend