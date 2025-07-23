serve port="4000":
    nix develop ~/code/rex --command rex serve -a {{port}}
