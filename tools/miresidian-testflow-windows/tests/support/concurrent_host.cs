// Test-only extra caller in the same native runtime as Invoke-Flow.
public static class TestFlowProbe {
    private static System.Threading.Thread worker;
    private static System.Exception error;
    public static void Start(string entered, string attempted, string result, string request) {
        worker = new System.Threading.Thread(() => {
            try {
                var deadline = System.DateTime.UtcNow.AddSeconds(10);
                while (!System.IO.File.Exists(entered)) {
                    if (System.DateTime.UtcNow > deadline) throw new System.TimeoutException("entered handshake");
                    System.Threading.Thread.Sleep(1);
                }
                System.IO.File.WriteAllText(attempted, "attempted");
                var response = MiresidianDllCaller.InvokeRequest(request);
                System.IO.File.WriteAllText(result, response);
            } catch (System.Exception exception) { error = exception; }
        });
        worker.IsBackground = true;
        worker.Start();
    }
    public static void Join() {
        if (!worker.Join(10000)) throw new System.TimeoutException("background invocation");
        if (error != null) throw new System.Exception("background invocation", error);
    }
}
