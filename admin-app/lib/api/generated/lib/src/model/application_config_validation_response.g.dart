// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_config_validation_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationConfigValidationResponse
    extends ApplicationConfigValidationResponse {
  @override
  final BuiltList<ConfigDiagnostic> diagnostics;
  @override
  final bool valid;

  factory _$ApplicationConfigValidationResponse([
    void Function(ApplicationConfigValidationResponseBuilder)? updates,
  ]) =>
      (ApplicationConfigValidationResponseBuilder()..update(updates))._build();

  _$ApplicationConfigValidationResponse._({
    required this.diagnostics,
    required this.valid,
  }) : super._();
  @override
  ApplicationConfigValidationResponse rebuild(
    void Function(ApplicationConfigValidationResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationConfigValidationResponseBuilder toBuilder() =>
      ApplicationConfigValidationResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationConfigValidationResponse &&
        diagnostics == other.diagnostics &&
        valid == other.valid;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, diagnostics.hashCode);
    _$hash = $jc(_$hash, valid.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationConfigValidationResponse')
          ..add('diagnostics', diagnostics)
          ..add('valid', valid))
        .toString();
  }
}

class ApplicationConfigValidationResponseBuilder
    implements
        Builder<
          ApplicationConfigValidationResponse,
          ApplicationConfigValidationResponseBuilder
        > {
  _$ApplicationConfigValidationResponse? _$v;

  ListBuilder<ConfigDiagnostic>? _diagnostics;
  ListBuilder<ConfigDiagnostic> get diagnostics =>
      _$this._diagnostics ??= ListBuilder<ConfigDiagnostic>();
  set diagnostics(ListBuilder<ConfigDiagnostic>? diagnostics) =>
      _$this._diagnostics = diagnostics;

  bool? _valid;
  bool? get valid => _$this._valid;
  set valid(bool? valid) => _$this._valid = valid;

  ApplicationConfigValidationResponseBuilder() {
    ApplicationConfigValidationResponse._defaults(this);
  }

  ApplicationConfigValidationResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _diagnostics = $v.diagnostics.toBuilder();
      _valid = $v.valid;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationConfigValidationResponse other) {
    _$v = other as _$ApplicationConfigValidationResponse;
  }

  @override
  void update(
    void Function(ApplicationConfigValidationResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationConfigValidationResponse build() => _build();

  _$ApplicationConfigValidationResponse _build() {
    _$ApplicationConfigValidationResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationConfigValidationResponse._(
            diagnostics: diagnostics.build(),
            valid: BuiltValueNullFieldError.checkNotNull(
              valid,
              r'ApplicationConfigValidationResponse',
              'valid',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'diagnostics';
        diagnostics.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationConfigValidationResponse',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
