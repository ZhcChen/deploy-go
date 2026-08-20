//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'restore_application_config_request.g.dart';

/// RestoreApplicationConfigRequest
///
/// Properties:
/// * [expectedVersion]
/// * [templateVersion]
/// * [version]
@BuiltValue()
abstract class RestoreApplicationConfigRequest implements Built<RestoreApplicationConfigRequest, RestoreApplicationConfigRequestBuilder> {
  @BuiltValueField(wireName: r'expected_version')
  int get expectedVersion;

  @BuiltValueField(wireName: r'template_version')
  String? get templateVersion;

  @BuiltValueField(wireName: r'version')
  int? get version;

  RestoreApplicationConfigRequest._();

  factory RestoreApplicationConfigRequest([void updates(RestoreApplicationConfigRequestBuilder b)]) = _$RestoreApplicationConfigRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(RestoreApplicationConfigRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<RestoreApplicationConfigRequest> get serializer => _$RestoreApplicationConfigRequestSerializer();
}

class _$RestoreApplicationConfigRequestSerializer implements PrimitiveSerializer<RestoreApplicationConfigRequest> {
  @override
  final Iterable<Type> types = const [RestoreApplicationConfigRequest, _$RestoreApplicationConfigRequest];

  @override
  final String wireName = r'RestoreApplicationConfigRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    RestoreApplicationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'expected_version';
    yield serializers.serialize(
      object.expectedVersion,
      specifiedType: const FullType(int),
    );
    if (object.templateVersion != null) {
      yield r'template_version';
      yield serializers.serialize(
        object.templateVersion,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.version != null) {
      yield r'version';
      yield serializers.serialize(
        object.version,
        specifiedType: const FullType.nullable(int),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    RestoreApplicationConfigRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required RestoreApplicationConfigRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'expected_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.expectedVersion = valueDes;
          break;
        case r'template_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.templateVersion = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(int),
          ) as int?;
          if (valueDes == null) continue;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  RestoreApplicationConfigRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = RestoreApplicationConfigRequestBuilder();
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
