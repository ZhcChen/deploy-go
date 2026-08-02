// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserResponse extends UserResponse {
  @override
  final String displayName;
  @override
  final String? email;
  @override
  final String id;
  @override
  final String identity;
  @override
  final String status;
  @override
  final String username;
  @override
  final int version;

  factory _$UserResponse([void Function(UserResponseBuilder)? updates]) =>
      (UserResponseBuilder()..update(updates))._build();

  _$UserResponse._({
    required this.displayName,
    this.email,
    required this.id,
    required this.identity,
    required this.status,
    required this.username,
    required this.version,
  }) : super._();
  @override
  UserResponse rebuild(void Function(UserResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserResponseBuilder toBuilder() => UserResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserResponse &&
        displayName == other.displayName &&
        email == other.email &&
        id == other.id &&
        identity == other.identity &&
        status == other.status &&
        username == other.username &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, displayName.hashCode);
    _$hash = $jc(_$hash, email.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, identity.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, username.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UserResponse')
          ..add('displayName', displayName)
          ..add('email', email)
          ..add('id', id)
          ..add('identity', identity)
          ..add('status', status)
          ..add('username', username)
          ..add('version', version))
        .toString();
  }
}

class UserResponseBuilder
    implements Builder<UserResponse, UserResponseBuilder> {
  _$UserResponse? _$v;

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

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _username;
  String? get username => _$this._username;
  set username(String? username) => _$this._username = username;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  UserResponseBuilder() {
    UserResponse._defaults(this);
  }

  UserResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _displayName = $v.displayName;
      _email = $v.email;
      _id = $v.id;
      _identity = $v.identity;
      _status = $v.status;
      _username = $v.username;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserResponse other) {
    _$v = other as _$UserResponse;
  }

  @override
  void update(void Function(UserResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserResponse build() => _build();

  _$UserResponse _build() {
    final _$result =
        _$v ??
        _$UserResponse._(
          displayName: BuiltValueNullFieldError.checkNotNull(
            displayName,
            r'UserResponse',
            'displayName',
          ),
          email: email,
          id: BuiltValueNullFieldError.checkNotNull(id, r'UserResponse', 'id'),
          identity: BuiltValueNullFieldError.checkNotNull(
            identity,
            r'UserResponse',
            'identity',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'UserResponse',
            'status',
          ),
          username: BuiltValueNullFieldError.checkNotNull(
            username,
            r'UserResponse',
            'username',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'UserResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
