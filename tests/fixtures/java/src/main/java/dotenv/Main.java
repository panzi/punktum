package dotenv;

import io.github.cdimascio.dotenv.Dotenv;
import java.util.Arrays;

public class Main {
    public static void main(String[] args) throws java.io.IOException, InterruptedException {
        var dotenv = Dotenv.configure();

        int index = 0;
        String filename = ".env";
        for (; index < args.length; ++ index) {
            var arg = args[index];
            if (arg.equals("--")) {
                ++ index;
                break;
            }

            if (arg.equals("--file")) {
                filename = args[index + 1];
                ++ index;
            } else if (arg.startsWith("-")) {
                throw new RuntimeException("illegal argument: " + arg);
            } else {
                break;
            }
        }

        var cmd = Arrays.copyOfRange(args, index, args.length);

        var env = dotenv.ignoreIfMalformed().filename(filename).load();

        var proc = new ProcessBuilder(cmd).
            redirectInput(ProcessBuilder.Redirect.INHERIT).
            redirectOutput(ProcessBuilder.Redirect.INHERIT).
            redirectError(ProcessBuilder.Redirect.INHERIT);

        var procenv = proc.environment();
        for (var entry : env.entries()) {
            procenv.put(entry.getKey(), entry.getValue());
        }

        var status = proc.start().waitFor();
        System.exit(status);
    }
}
