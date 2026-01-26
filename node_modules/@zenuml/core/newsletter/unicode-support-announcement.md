# 🌐 ZenUML Now Supports Unicode Characters!

*September 2025 Newsletter*

---

We're excited to announce a major enhancement to ZenUML: **full Unicode character support** for identifiers! You can now create sequence diagrams using native characters from Chinese, Japanese, Korean, Arabic, Cyrillic, and many other languages.

## ✨ What's New

### Native Language Support

No more workarounds or quoted strings for basic identifiers. You can now write ZenUML diagrams in your native language:

**Chinese Example:**
```zenuml
用户 订单服务 数据库

用户.登录()
订单服务.创建订单()
数据库.保存数据()
return 成功
```

**Japanese Example:**
```zenuml
ユーザー システム データベース

ユーザー.ログイン()
システム.認証()
データベース.検索()
```

**Mixed Languages:**
```zenuml
UserService 数据库 CacheManager

UserService.获取用户()
数据库.query("SELECT * FROM users")
CacheManager.缓存结果()
```

### Supported Languages

The new Unicode support includes:
- **Chinese** (Simplified & Traditional)
- **Japanese** (Hiragana, Katakana, Kanji)
- **Korean** (Hangul)
- **Arabic**
- **Cyrillic** (Russian, Bulgarian, etc.)
- **And many more Unicode languages!**

## 🔧 Technical Implementation

### Grammar Enhancement

We've updated the ANTLR lexer to use Unicode property classes:

```antlr
ID : [\p{L}_] [\p{L}\p{Nd}_]* ;
```

- `\p{L}` matches any Unicode letter
- `\p{Nd}` matches any Unicode decimal digit
- Maintains backward compatibility with ASCII identifiers

### Backward Compatibility

**All existing ZenUML code continues to work unchanged.** This is a purely additive feature that expands what's possible without breaking existing diagrams.

## 📋 Usage Guidelines

### Simple Identifiers
Use Unicode characters directly for participant names, method names, and variables:
- ✅ `用户.登录()`
- ✅ `ユーザー.データ取得()`
- ✅ `사용자.인증()`

### With Spaces
For identifiers containing spaces, use quotes:
- ✅ `"用户 服务".获取信息()`
- ✅ `"Order Management".processOrder()`

### Rules
- Must start with a Unicode letter or underscore
- Can contain Unicode letters, numbers, and underscores
- Keywords (if, while, return) remain in English

## 📊 Before vs After: See the Difference

| Scenario | Before (Workarounds) | After (Native Unicode) |
|----------|---------------------|------------------------|
| **Chinese participants** | `"用户" "订单服务" "数据库"` | `用户 订单服务 数据库` |
| **Japanese methods** | `User."ログイン"()` | `ユーザー.ログイン()` |
| **Mixed languages** | `"UserService" "数据库" "CacheManager"` | `UserService 数据库 CacheManager` |
| **Korean business logic** | `"사용자"."인증"()` | `사용자.인증()` |
| **Arabic identifiers** | `"المستخدم"."تسجيل_الدخول"()` | `المستخدم.تسجيل_الدخول()` |

### Real Impact Examples

**E-commerce Team (Chinese):**
```zenuml
// Before: Mixed English/quoted Chinese
User "订单服务" Database
User."创建订单"()
"订单服务".validateOrder()

// After: Pure Chinese business language
用户 订单服务 数据库
用户.创建订单()
订单服务.验证订单()
```

**Banking System (Japanese):**
```zenuml
// Before: Awkward English mapping
Customer "AccountService" "TransactionDB"
Customer."残高照会"()

// After: Natural Japanese terminology
顧客 口座サービス 取引データベース
顧客.残高照会()
口座サービス.残高取得()
```

## 🚀 Try It Now

Visit https://app.zenuml.com

---

**Happy Diagramming in Your Language!** 🎉

*Have feedback or found an issue? [Open an issue on GitHub](https://github.com/mermaid-js/zenuml-core/issues) or join our community discussions.*