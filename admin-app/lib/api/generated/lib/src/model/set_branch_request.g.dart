// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'set_branch_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SetBranchRequest extends SetBranchRequest {
  @override
  final String branch;
  @override
  final int version;

  factory _$SetBranchRequest([
    void Function(SetBranchRequestBuilder)? updates,
  ]) => (SetBranchRequestBuilder()..update(updates))._build();

  _$SetBranchRequest._({required this.branch, required this.version})
    : super._();
  @override
  SetBranchRequest rebuild(void Function(SetBranchRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  SetBranchRequestBuilder toBuilder() =>
      SetBranchRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SetBranchRequest &&
        branch == other.branch &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, branch.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SetBranchRequest')
          ..add('branch', branch)
          ..add('version', version))
        .toString();
  }
}

class SetBranchRequestBuilder
    implements Builder<SetBranchRequest, SetBranchRequestBuilder> {
  _$SetBranchRequest? _$v;

  String? _branch;
  String? get branch => _$this._branch;
  set branch(String? branch) => _$this._branch = branch;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SetBranchRequestBuilder() {
    SetBranchRequest._defaults(this);
  }

  SetBranchRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _branch = $v.branch;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SetBranchRequest other) {
    _$v = other as _$SetBranchRequest;
  }

  @override
  void update(void Function(SetBranchRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SetBranchRequest build() => _build();

  _$SetBranchRequest _build() {
    final _$result =
        _$v ??
        _$SetBranchRequest._(
          branch: BuiltValueNullFieldError.checkNotNull(
            branch,
            r'SetBranchRequest',
            'branch',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'SetBranchRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
