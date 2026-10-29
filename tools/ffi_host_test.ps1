# Gerçek sürüm kitaplığını başka bir dilden çağırarak bellek ve hata sınırını sınar.
param([string]$LibraryPath)
$ErrorActionPreference = 'Stop'
if (-not $LibraryPath) { $LibraryPath = Join-Path $PSScriptRoot '..\src\target\release\matrix_core.dll' }
$LibraryPath = (Resolve-Path -LiteralPath $LibraryPath).Path
$matrixDllLiteral = $LibraryPath.Replace('\', '\\').Replace('"', '\"')
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class MatrixNativeTest {
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern uint matrix_abi_version();
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_create(uint version,uint width,uint height,ulong seed,IntPtr font,UIntPtr length,out ulong result);
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_set_active(ulong handle,uint active);
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_update(ulong handle,ulong elapsed,byte hour,byte minute,byte second);
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_render_rgba(ulong handle,IntPtr data,UIntPtr length,UIntPtr stride);
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_destroy(ulong handle);
    [DllImport("$matrixDllLiteral", CallingConvention=CallingConvention.Cdecl)]
    public static extern int matrix_test_panic();
}
"@
function Assert-MatrixResult([int]$Actual, [int]$Expected, [string]$Label) {
    if ($Actual -ne $Expected) { throw "$Label : beklenen $Expected, alınan $Actual" }
}
if ([MatrixNativeTest]::matrix_abi_version() -ne 1) { throw 'Çağrı sürümü farklı.' }
[ulong]$matrixId = 0
Assert-MatrixResult ([MatrixNativeTest]::matrix_create(1,16,8,41,[IntPtr]::Zero,[UIntPtr]::Zero,[ref]$matrixId)) 0 'Oluşturma'
$matrixMemory = [Runtime.InteropServices.Marshal]::AllocHGlobal(8*80+2)
try {
    $matrixBytes = [byte[]]::new(8*80+2)
    [Array]::Fill[byte]($matrixBytes,77)
    [Runtime.InteropServices.Marshal]::Copy($matrixBytes,0,$matrixMemory,$matrixBytes.Length)
    Assert-MatrixResult ([MatrixNativeTest]::matrix_set_active($matrixId,1)) 0 'Başlatma'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_update($matrixId,40000,12,34,56)) 0 'Zaman'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_render_rgba($matrixId,[IntPtr]::Add($matrixMemory,1),[UIntPtr]640,[UIntPtr]80)) 0 'Çizim'
    [Runtime.InteropServices.Marshal]::Copy($matrixMemory,$matrixBytes,0,$matrixBytes.Length)
    if ($matrixBytes[0] -ne 77 -or $matrixBytes[-1] -ne 77) { throw 'Görüntü alanı dışına yazıldı.' }
    for ($matrixRow=0; $matrixRow -lt 8; $matrixRow++) {
        for ($matrixByte=64; $matrixByte -lt 80; $matrixByte++) {
            if ($matrixBytes[1+$matrixRow*80+$matrixByte] -ne 77) { throw 'Satır boşluğu değişti.' }
        }
    }
    Assert-MatrixResult ([MatrixNativeTest]::matrix_render_rgba($matrixId,$matrixMemory,[UIntPtr]1,[UIntPtr]80)) 3 'Kısa alan'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_test_panic()) 6 'Yakalanabilir hata'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_update($matrixId,40000,12,34,57)) 0 'Hatadan sonra çalışma'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_destroy($matrixId)) 0 'Kapatma'
    Assert-MatrixResult ([MatrixNativeTest]::matrix_destroy($matrixId)) 2 'Tekrar kapatma'
    Write-Output 'Sürüm kitaplığı: bellek sınırları, hata yakalama ve kapatma başarılı.'
} finally {
    [void][MatrixNativeTest]::matrix_destroy($matrixId)
    [Runtime.InteropServices.Marshal]::FreeHGlobal($matrixMemory)
}

