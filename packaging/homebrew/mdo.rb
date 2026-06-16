class Mdo < Formula
  desc "Convert Markdown to standalone HTML5 documents"
  homepage "https://maphew.github.io/mdo/"
  version "0.3.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/maphew/mdo/releases/download/v0.3.0/mdo-universal-apple-darwin.tar.gz"
    sha256 "e3e8f4fd46b4bfed8b8816ebdcd6d462a56a8bb17dac6d7de7ae7f4447de9ad9"
  end

  on_linux do
    on_intel do
      url "https://github.com/maphew/mdo/releases/download/v0.3.0/mdo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "69d855334208b1b3021de240dbe591e733dd4425d951ab7dbedfd46ac0d8902d"
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
