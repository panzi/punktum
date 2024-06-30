package main

import (
	"fmt"
	"os"
	"os/exec"
	"log"
	"strings"
	"github.com/compose-spec/compose-go/v2/dotenv"
)

func main() {
	path := os.Getenv("DOTENV_CONFIG_PATH")
	if path == "" {
		path = ".env"
	}

	replace := false

	i := 1
	for i < len(os.Args) {
		arg := os.Args[i]

		if arg == "--file" || arg == "-f" {
			i += 1
			path = os.Args[i]
		} else if arg == "--replace" || arg == "-r" {
			replace = true
		} else if arg == "--" {
			i += 1
			break
		} else if strings.HasPrefix(arg, "-") {
			log.Fatal("illegal argument: ", arg)
		} else {
			break
		}

		i += 1
	}

	prog := os.Args[i]
	args := os.Args[i + 1:]

	progPath, err := exec.LookPath(prog)
	if err != nil {
		log.Fatal(err)
	}
	cmd := exec.Command(progPath, args...)

	if replace {
		environ := os.Environ()
		currentEnv := make(map[string]string)
		for _, pair := range environ {
			parts := strings.SplitN(pair, "=", 2)
			currentEnv[parts[0]] = parts[1]
		}

		env, err := dotenv.GetEnvFromFile(currentEnv, []string{ path })
		if err != nil {
			log.Fatal(err)
		}

		cmdEnv := make([]string, len(env))
		for key, value := range env {
			cmdEnv = append(cmdEnv, fmt.Sprintf("%s=%s", key, value))
		}

		cmd.Env = cmdEnv
	} else {
		err := dotenv.Load(path)
		if err != nil {
			log.Fatal(err)
		}
	}

	if err := cmd.Run(); err != nil {
		log.Fatal(err)
	}
}
