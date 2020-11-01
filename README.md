# R-exec
The tool allows one to run executables remotely.
The command is posted by the HTTP POST method containing a JSON-based description.
The stdout of the remote process is streamed to the HTTP response. The HTTP connection is kept open until
the remote process is running.
Closing a connection will stop a remote process.

## Request format
The request has a json format.

### alias
Is a unique short name of the command.
If process with the same alias is already running,
the error wil be returned.

### cmd
A fully qualified or a short name of the command.
If a short name is provided it is up to the operating system
to find the executable in the PATH variable.

### cwd (optional)
A working directory for the command.
If not set the `.` will be used as a current directory for the command.
 

### args (optional)
A list of arguments to be passed to the command

### envs (optional)
A list of environment variables to be set before execution of the command.

### Example of the command message 
This command will run a shell, which will execute 
the ls command with options -rtl.
The request is equal to: `sh -c "ls -rtl"` 

```
{
    "alias" : "do_ls",
    "cmd": "sh",
    "args": [
        "-c",
        "ls -rtl"
    ],
    "cwd": ".",
    "envs": {
        "PATH": "/bin",
        "SECRET_KEY": "QWE_YUI_345_GHJ_789"
    }
}
```

## Http Return codes
### 500 Internal Server Error
Is returned if any internal error occur in the executor code itself.
The response will contain an error text if any.

### 404 Non Found
Is returned if the process fail to start for any reason,
and the stdout was not created.
The response will contain an error text if any.

### 409 Conflict
Is returned if the command with equal alias is already running.

