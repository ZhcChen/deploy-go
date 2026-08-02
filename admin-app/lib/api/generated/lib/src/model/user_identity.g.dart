// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_identity.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserIdentity extends UserIdentity {
  @override
  final String displayName;
  @override
  final String? email;
  @override
  final String id;
  @override
  final String identity;
  @override
  final String username;

  factory _$UserIdentity([void Function(UserIdentityBuilder)? updates]) =>
      (UserIdentityBuilder()..update(updates))._build();

  _$UserIdentity._({
    required this.displayName,
    this.email,
    required this.id,
    required this.identity,
    required this.username,
  }) : super._();
  @override
  UserIdentity rebuild(void Function(UserIdentityBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserIdentityBuilder toBuilder() => UserIdentityBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserIdentity &&
        displayName == other.displayName &&
        email == other.email &&
        id == other.id &&
        identity == other.identity &&
        username == other.username;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, displayName.hashCode);
    _$hash = $jc(_$hash, email.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, identity.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UserIdentity')
          ..add('displayName', displayName)
          ..add('email', email)
          ..add('id', id)
          ..add('identity', identity)
          ..add('username', username))
        .toString();
  }
}

class UserIdentityBuilder
    implements Builder<UserIdentity, UserIdentityBuilder> {
  _$UserIdentity? _$v;

  String? _displayName;
  String? get displayName => _$this._displayName;
  set displayName(String? displayName) => _$this._displayName = displayName;

  String? _email;
  String? get email => _$this._email;
  set email(String? email) => _$this._email = email;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _identity;
  String? get identity => _$this._identity;
  set identity(String? identity) => _$this._identity = identity;

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  UserIdentityBuilder() {
    UserIdentity._defaults(this);
  }

  UserIdentityBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _displayName = $v.displayName;
      _email = $v.email;
      _id = $v.id;
      _identity = $v.identity;
      _username = $v.username;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserIdentity other) {
    _$v = other as _$UserIdentity;
  }

  @override
  void update(void Function(UserIdentityBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserIdentity build() => _build();

  _$UserIdentity _build() {
    final _$result =
        _$v ??
        _$UserIdentity._(
          displayName: BuiltValueNullFieldError.checkNotNull(
            displayName,
            r'UserIdentity',
            'displayName',
          ),
          email: email,
          id: BuiltValueNullFieldError.checkNotNull(id, r'UserIdentity', 'id'),
          identity: BuiltValueNullFieldError.checkNotNull(
            identity,
            r'UserIdentity',
            'identity',
          ),
          username: BuiltValueNullFieldError.checkNotNull(
            username,
            r'UserIdentity',
            'username',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
