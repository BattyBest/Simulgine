let
    pkgs = import <nixpkgs>{ };
in
    pkgs.mkShell {
    	packages = with pkgs; [ rustup rust-analyzer gh clang llvmPackages.bintools ];
    }
