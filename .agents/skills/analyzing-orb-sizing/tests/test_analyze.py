import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).parents[1] / "scripts" / "analyze.py"
SPEC = importlib.util.spec_from_file_location("analyze", MODULE_PATH)
assert SPEC and SPEC.loader
analyze = importlib.util.module_from_spec(SPEC)
sys.modules["analyze"] = analyze
SPEC.loader.exec_module(analyze)


def thread_with_result(
    result: str,
    exit_code: int = 0,
    command: str = "representative-workload",
    *,
    current_schema: bool = False,
    tool_name: str = "shell_command",
) -> dict:
    tool_result = (
        {
            "type": "tool_result",
            "toolUseID": "shell-1",
            "status": "done" if exit_code == 0 else "error",
            "output": result,
        }
        if current_schema
        else {
            "type": "tool_result",
            "toolUseID": "shell-1",
            "run": {
                "status": "done",
                "result": {"output": result, "exitCode": exit_code},
            },
        }
    )
    return {
        "messages": [
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "shell-1",
                        "name": tool_name,
                        "input": {"cmd" if tool_name == "Bash" else "command": command},
                    }
                ],
            },
            {
                "role": "user",
                "content": [
                    tool_result
                ],
            }
        ]
    }


class AssessTests(unittest.TestCase):
    def test_pressure_at_largest_size_requests_manual_escalation(self) -> None:
        result = analyze.assess(
            thread_with_result("process exited with status 137", exit_code=137),
            "a0.large",
        )
        self.assertEqual(result.verdict, "under-sized")
        self.assertEqual(result.recommended_size, "a0.large")
        self.assertIn("largest documented size", result.missing_evidence[0])

    def test_current_amp_bash_result_is_analyzed(self) -> None:
        result = analyze.assess(
            thread_with_result(
                "process exited with status 137",
                exit_code=137,
                current_schema=True,
                tool_name="Bash",
            ),
            "a0.small",
        )
        self.assertEqual(result.verdict, "under-sized")
        self.assertEqual(result.recommended_size, "a0.medium")

    def test_current_amp_structured_text_output_is_analyzed(self) -> None:
        thread = thread_with_result("unused", current_schema=True, tool_name="Bash")
        thread["messages"][1]["content"][0]["output"] = [
            {"type": "text", "text": "process exited with status 137"}
        ]
        thread["messages"][1]["content"][0]["status"] = "error"
        result = analyze.assess(thread, "a0.small")
        self.assertEqual(result.verdict, "under-sized")

    def test_oom_is_under_sized(self) -> None:
        result = analyze.assess(
            thread_with_result("process exited with status 137", exit_code=137),
            "a0.small",
        )
        self.assertEqual(result.verdict, "under-sized")
        self.assertEqual(result.recommended_size, "a0.medium")
        self.assertEqual(result.confidence, "high")

    def test_prose_is_not_treated_as_evidence(self) -> None:
        thread = {
            "messages": [
                {
                    "role": "user",
                    "content": [{"type": "text", "text": "Did this build OOM?"}],
                }
            ]
        }
        result = analyze.assess(thread, "a0.small")
        self.assertEqual(result.verdict, "insufficient-evidence")

    def test_non_shell_tool_output_is_not_treated_as_evidence(self) -> None:
        thread = thread_with_result("process exited with status 137", exit_code=137)
        thread["messages"][0]["content"][0]["name"] = "read_thread"
        result = analyze.assess(thread, "a0.small")
        self.assertEqual(result.verdict, "insufficient-evidence")

    def test_successful_discussion_of_oom_is_not_pressure(self) -> None:
        result = analyze.assess(
            thread_with_result("Prior analyzer said OOM and exit status 137"),
            "a0.small",
        )
        self.assertEqual(result.verdict, "insufficient-evidence")

    def test_absent_telemetry_does_not_imply_over_sized(self) -> None:
        result = analyze.assess(thread_with_result("Finished tests successfully"), "a0.large")
        self.assertEqual(result.verdict, "insufficient-evidence")
        self.assertEqual(result.recommended_size, "a0.large")

    def test_trivial_measurement_is_insufficient(self) -> None:
        time_output = """
Percent of CPU this job got: 10%
Elapsed (wall clock) time (h:mm:ss or m:ss): 0:01.00
Maximum resident set size (kbytes): 1024
"""
        result = analyze.assess(
            thread_with_result(time_output, command="/usr/bin/time -v true"),
            "a0.large",
        )
        self.assertEqual(result.verdict, "insufficient-evidence")

    def test_representative_low_usage_recommends_smaller_size(self) -> None:
        time_output = """
Percent of CPU this job got: 85%
Elapsed (wall clock) time (h:mm:ss or m:ss): 1:10.00
Maximum resident set size (kbytes): 1048576
"""
        result = analyze.assess(
            thread_with_result(time_output, command="/usr/bin/time -v cargo test"),
            "a0.medium",
        )
        self.assertEqual(result.verdict, "over-sized")
        self.assertEqual(result.recommended_size, "a0.small")

    def test_high_memory_measurement_recommends_larger_size(self) -> None:
        time_output = """
Percent of CPU this job got: 190%
Elapsed (wall clock) time (h:mm:ss or m:ss): 2:10.00
Maximum resident set size (kbytes): 3700000
"""
        result = analyze.assess(
            thread_with_result(time_output, command="/usr/bin/time -v cargo test"),
            "a0.small",
        )
        self.assertEqual(result.verdict, "under-sized")
        self.assertEqual(result.recommended_size, "a0.medium")


if __name__ == "__main__":
    unittest.main()
