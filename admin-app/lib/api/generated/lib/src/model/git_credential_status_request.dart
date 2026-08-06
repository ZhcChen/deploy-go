//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'git_credential_status_request.g.dart';

/// GitCredentialStatusRequest
///
/// Properties:
/// * [status]
/// * [version]
@BuiltValue()
abstract class GitCredentialStatusRequest implements Built<GitCredentialStatusRequest, GitCredentialStatusRequestBuilder> {
  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'version')
  int get version;

  GitCredentialStatusRequest._();

  factory GitCredentialStatusRequest([void updates(GitCredentialStatusRequestBuilder b)]) = _$GitCredentialStatusRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(GitCredentialStatusRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<GitCredentialStatusRequest> get serializer => _$GitCredentialStatusRequestSerializer();
}

class _$GitCredentialStatusRequestSerializer implements PrimitiveSerializer<GitCredentialStatusRequest> {
  @override
  final Iterable<Type> types = const [GitCredentialStatusRequest, _$GitCredentialStatusRequest];

  @override
  final String wireName = r'GitCredentialStatusRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    GitCredentialStatusRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    GitCredentialStatusRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required GitCredentialStatusRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
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
  GitCredentialStatusRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = GitCredentialStatusRequestBuilder();
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
