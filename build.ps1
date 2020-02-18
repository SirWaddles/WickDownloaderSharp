cd native

function BuildEnv($toolchain) {
	if ($toolchain -eq 'x64') {
		$targetParam = "x86_64-pc-windows-msvc";
		$destArch = "win10-x64";
	}
	if ($toolchain -eq 'x86') {
		$targetParam = "i686-pc-windows-msvc";
		$destArch = "win10-x86";
	}
	$params = @("build", "--target", $targetParam, "--release");
	cargo $params
	New-Item "../platform/runtimes/$($destArch)/native" -ItemType Directory -Force
	Copy-Item "target/$($targetParam)/release/wick_downloader.dll" "../platform/runtimes/$($destArch)/native/wick_downloader.dll"
}

BuildEnv x64
BuildEnv x86

cd ..



