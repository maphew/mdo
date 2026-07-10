class Mdo < Formula
  desc "Convert Markdown to standalone HTML5 documents"
  homepage "https://maphew.github.io/mdo/"
  version "0.5.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/maphew/mdo/releases/download/v0.5.0/mdo-universal-apple-darwin.tar.gz"
    sha256 "3c9f4bd72e67b43abe9a4c597d9fedcd08174a52385e61f6cff2c9c0cfe25b1d"
  end

  on_linux do
    on_intel do
      url "https://github.com/maphew/mdo/releases/download/v0.5.0/mdo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "b9292b3a98dca7dac401f100c02100101f8c738fa457bf4f774b67a8e5a3c4e7"
    end
  end

  def install
    bin.install "mdo"
    bin.install "mdo-open"
    bin.install "mdo-setup" if OS.linux?
    doc.install "CHANGELOG.md", "README.md"
    license.install "LICENSE-APACHE", "LICENSE-MIT"
  end

  test do
    (testpath/"input.md").write("# Hello\n")
    system bin/"mdo", testpath/"input.md", "--output", testpath/"output.html"
    assert_match "<h1>Hello</h1>", (testpath/"output.html").read
  end
end
