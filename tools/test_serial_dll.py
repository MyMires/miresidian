"""手动调用串口 DLL：编辑 function、config、input 后直接运行此文件。"""

from pathlib import Path

from c_abi_shared import invoke


PROJECT_ROOT = Path(__file__).resolve().parents[1]
# 目标为实际 Windows DLL；须使用同位数 Windows Python，或 Wine 中的 Windows Python。
DLL_PATH = PROJECT_ROOT / "dist" / "串口通信" / "windows-x64" / "miresidian_serial.dll"

# ===== 手动测试输入：只编辑以下三个变量 =====
function = "GetFunctions"
config: list[str] = []
input: list[str] = []

# InitDevice 示例：
# function = "InitDevice"
# config = ["COM3", "9600", "8", "None", "1", "None", "1000", "1000"]
# input = []


if __name__ == "__main__":
    result_json = invoke(DLL_PATH, function, config, input)
    print(result_json)
