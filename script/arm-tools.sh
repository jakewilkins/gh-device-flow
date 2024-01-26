function swp() {
		local tmp=${1##*/}
		mv "$1" "${tmp%.*}$2"
}

function reset() {
	echo "Exiting"
	if [ -f ./Cargo.bak ]; then
		echo "Resetting Cargo.toml"
		swp Cargo.toml .arm
		swp Cargo.bak .toml
	else
		echo "Not resetting Cargo.toml"
	fi
}

function setup() {
	echo "Setting up Cargo.toml for arm build"
	swp Cargo.toml .bak
	swp Cargo.arm .toml
}

