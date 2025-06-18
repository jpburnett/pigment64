import pytest
import pigment64
import os

# Get the absolute path to the directory where this test script is located.
# This makes the tests runnable from any directory.
TESTS_DIR = os.path.dirname(os.path.abspath(__file__))

def test_extract_palette_from_png_bytes_success():
    """
    Tests that a valid PNG with a palette returns the correct palette bytes.
    It compares the output against the known-good 'ci4.tlut.bin' file.
    """
    # Arrange: Construct the full path to the test asset files
    png_path = os.path.join(TESTS_DIR, "ci4.png")
    expected_palette_path = os.path.join(TESTS_DIR, "ci4.tlut.bin")

    # Check that test files exist before trying to open them
    if not os.path.exists(png_path):
        pytest.fail(f"Test asset not found: {png_path}")
    if not os.path.exists(expected_palette_path):
        pytest.fail(f"Expected result file not found: {expected_palette_path}")

    with open(png_path, "rb") as f:
        png_binary_data = f.read()

    with open(expected_palette_path, "rb") as f:
        expected_palette_bytes = f.read()

    # Act: Call the function from the Rust library
    actual_palette_bytes = pigment64.extract_palette_from_png_bytes(png_binary_data)

    # Assert: Check that the actual output matches the expected output
    assert actual_palette_bytes == expected_palette_bytes
    assert len(actual_palette_bytes) == 32