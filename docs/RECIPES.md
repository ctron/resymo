# Recipes

## Get the pending number of updates

```yaml
$schema: "https://raw.githubusercontent.com/ctron/resymo/main/deploy/config/schema.json"
collectors: 
  exec: 
    items:
      pending_updates:
        period: 1h
        command: bash
        args:
          - -c
          - |
            apt update &> /dev/null;
            apt list --upgradable -qq | wc -l
        discovery:
          - name: Pending updates
            state_class: measurement
            value_template: '{{ value_json.stdout }}'
```
