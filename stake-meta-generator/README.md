# Stake Meta Generator

Before running the generator, raise the open-file limit on the current shell:

```sh
sudo prlimit --pid $$ --nofile=1000000:1000000
```

The generator inherits this limit when it is launched from the same shell.
