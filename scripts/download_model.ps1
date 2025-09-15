# PowerShell script to download Qwen2.5-Coder-0.5B GGUF model

Write-Host "Downloading Qwen2.5-Coder-0.5B Q4 quantized model..." -ForegroundColor Green

# Create models directory
$modelsDir = "models"
if (!(Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir
    Write-Host "Created models directory"
}

# Download the Q4_K_M quantized model (491 MB - good balance of size and quality)
$modelUrl = "https://huggingface.co/Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF/resolve/main/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf"
$modelPath = Join-Path $modelsDir "qwen2.5-coder-0.5b-instruct-q4_k_m.gguf"

if (Test-Path $modelPath) {
    Write-Host "Model already exists at: $modelPath" -ForegroundColor Yellow
    $size = (Get-Item $modelPath).Length / 1MB
    Write-Host "Size: $([math]::Round($size, 2)) MB"
} else {
    Write-Host "Downloading from: $modelUrl"
    Write-Host "This will download approximately 491 MB..."
    
    try {
        Invoke-WebRequest -Uri $modelUrl -OutFile $modelPath -UseBasicParsing
        Write-Host "Model downloaded successfully to: $modelPath" -ForegroundColor Green
    } catch {
        Write-Host "Error downloading model: $_" -ForegroundColor Red
        Write-Host "You may need to install huggingface-cli and use:" -ForegroundColor Yellow
        Write-Host "huggingface-cli download Qwen/Qwen2.5-Coder-0.5B-Instruct-GGUF qwen2.5-coder-0.5b-instruct-q4_k_m.gguf --local-dir ./models"
    }
}

Write-Host "`nModel Information:" -ForegroundColor Cyan
Write-Host "- Size: ~491 MB (Q4_K_M quantization)"
Write-Host "- Parameters: 0.5B"
Write-Host "- Context: 32K tokens"
Write-Host "- Type: Instruction-tuned for code"