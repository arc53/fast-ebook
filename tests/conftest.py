import pytest
from pathlib import Path

FIXTURES = Path(__file__).parent / "fixtures"


@pytest.fixture
def epub2_path():
    return str(FIXTURES / "minimal_epub2.epub")


@pytest.fixture
def epub3_path():
    return str(FIXTURES / "minimal_epub3.epub")


@pytest.fixture
def multi_chapter_path():
    return str(FIXTURES / "multi_chapter.epub")


@pytest.fixture
def nested_toc_path():
    return str(FIXTURES / "nested_toc.epub")
