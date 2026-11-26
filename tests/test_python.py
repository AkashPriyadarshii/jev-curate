"""Minimal offline smoke test for the jev_curate Python extension."""

import unittest


class TestJevCuratePython(unittest.TestCase):
    def test_import_and_init(self):
        import jev_curate

        self.assertTrue(hasattr(jev_curate, "PyJevCurator"))
        curator = jev_curate.PyJevCurator(api_key="test-key", preset="pretrain")
        self.assertIsNotNone(curator)

    def test_invalid_preset_raises(self):
        import jev_curate

        with self.assertRaises(ValueError):
            jev_curate.PyJevCurator(api_key="test-key", preset="nonexistent-preset")


if __name__ == "__main__":
    unittest.main()
