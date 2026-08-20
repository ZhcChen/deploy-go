//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'config_diagnostic.g.dart';

/// ConfigDiagnostic
///
/// Properties:
/// * [code]
/// * [column]
/// * [line]
/// * [message]
/// * [path]
@BuiltValue()
abstract class ConfigDiagnostic implements Built<ConfigDiagnostic, ConfigDiagnosticBuilder> {
  @BuiltValueField(wireName: r'code')
  String get code;

  @BuiltValueField(wireName: r'column')
  int get column;

  @BuiltValueField(wireName: r'line')
  int get line;

  @BuiltValueField(wireName: r'message')
  String get message;

  @BuiltValueField(wireName: r'path')
  String get path;

  ConfigDiagnostic._();

  factory ConfigDiagnostic([void updates(ConfigDiagnosticBuilder b)]) = _$ConfigDiagnostic;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ConfigDiagnosticBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ConfigDiagnostic> get serializer => _$ConfigDiagnosticSerializer();
}

class _$ConfigDiagnosticSerializer implements PrimitiveSerializer<ConfigDiagnostic> {
  @override
  final Iterable<Type> types = const [ConfigDiagnostic, _$ConfigDiagnostic];

  @override
  final String wireName = r'ConfigDiagnostic';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ConfigDiagnostic object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'code';
    yield serializers.serialize(
      object.code,
      specifiedType: const FullType(String),
    );
    yield r'column';
    yield serializers.serialize(
      object.column,
      specifiedType: const FullType(int),
    );
    yield r'line';
    yield serializers.serialize(
      object.line,
      specifiedType: const FullType(int),
    );
    yield r'message';
    yield serializers.serialize(
      object.message,
      specifiedType: const FullType(String),
    );
    yield r'path';
    yield serializers.serialize(
      object.path,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ConfigDiagnostic object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ConfigDiagnosticBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'code':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.code = valueDes;
          break;
        case r'column':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.column = valueDes;
          break;
        case r'line':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.line = valueDes;
          break;
        case r'message':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.message = valueDes;
          break;
        case r'path':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.path = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ConfigDiagnostic deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ConfigDiagnosticBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
