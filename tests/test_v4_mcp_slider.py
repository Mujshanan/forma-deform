import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "tools"))
from v4_mcp_slider import bone_angle, slider_frame


class SliderMappingTests(unittest.TestCase):
    def test_exclusive_endpoints(self):
        self.assertEqual(slider_frame(-45, -45, 45, 0, 20), 0)
        self.assertEqual(slider_frame(0, -45, 45, 0, 20), 10)
        self.assertEqual(slider_frame(45, -45, 45, 0, 20), 19)

    def test_inclusive_endpoint(self):
        self.assertEqual(slider_frame(45, -45, 45, 0, 20, "inclusive"), 20)

    def test_reversed_angle_range(self):
        self.assertEqual(slider_frame(-45, 45, -45, 0, 20), 19)
        self.assertEqual(slider_frame(45, 45, -45, 0, 20), 0)

    def test_clamps_out_of_range_angles(self):
        self.assertEqual(slider_frame(-100, -45, 45, 0, 20), 0)
        self.assertEqual(slider_frame(100, -45, 45, 0, 20), 19)

    def test_reads_numeric_bone_id_from_verified_bone_info_shape(self):
        class FakeClient:
            def command(self, command, params):
                self.call = (command, params)
                return {"frame": 3, "armatures": [
                    {"layer": 7, "bones": [{"id": 12, "angle": -22.5}]}
                ]}
        client = FakeClient()
        self.assertEqual(bone_angle(client, 7, 12, 3), -22.5)
        self.assertEqual(client.call, ("bone.info", {"layer": 7, "frame": 3}))

    def test_rejects_invalid_ranges(self):
        with self.assertRaises(ValueError):
            slider_frame(0, 0, 0, 0, 20)
        with self.assertRaises(ValueError):
            slider_frame(0, -45, 45, 20, 20)


if __name__ == "__main__":
    unittest.main()
