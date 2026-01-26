Assignment occurs in the following contexts:
- Non-self Sync Message `ret = A.m()` or `T ret = A.m()`
- Self Sync Message `ret = m()` or `T ret = m()`
- Creation `ret = new A()` or `T ret = new A()`

For self sync message, the assignment is rendered with the message itself.

For non-self sync message and creation, the assignment is rendered as a return statement. If the Type presents, the return message would be like `t:T`. In such cases, we need to provide positions for type and assignee separately.
